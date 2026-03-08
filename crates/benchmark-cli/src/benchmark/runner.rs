use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::cli::Args;
use crate::output::progress_line;
use crate::resp::consume_response;
use crate::workload::BenchRun;

use super::connection::{
    authenticate_and_select_for_idle, open_connection, setup_connection_state,
};
use super::model::{BenchResult, ClientPlan, Progress, RandomSource, Shared, WorkerStats};
use super::request::build_request_group;
use super::stats::{build_cumulative_distribution, ns_to_ms, percentile_ms};

pub async fn run_single_benchmark(args: &Args, run: BenchRun) -> Result<BenchResult, String> {
    let clients = run.clients.min(run.requests as usize).max(1);
    let base = run.requests / clients as u64;
    let extra = (run.requests % clients as u64) as usize;
    let thread_count = args.thread_count().min(clients).max(1);

    let shared = Arc::new(Shared {
        host: args.host.clone(),
        port: args.port,
        user: args.user.clone(),
        password: args.password.clone(),
        run,
        strict: args.strict,
    });

    let mut shards = vec![Vec::new(); thread_count];
    for client_id in 0..clients {
        let quota = base + u64::from(client_id < extra);
        if quota == 0 {
            continue;
        }
        shards[client_id % thread_count].push(ClientPlan {
            client_id: client_id as u64,
            quota,
        });
    }

    let progress = Arc::new(Progress {
        completed: AtomicU64::new(0),
        finished: AtomicBool::new(false),
    });

    let reporter = if args.quiet || args.csv {
        None
    } else {
        Some(spawn_progress_reporter(
            shared.run.name.clone(),
            shared.run.requests,
            Arc::clone(&progress),
        ))
    };

    let start = Instant::now();
    let mut handles = Vec::with_capacity(thread_count);
    for (thread_index, shard) in shards.into_iter().enumerate() {
        if shard.is_empty() {
            continue;
        }

        let cfg = Arc::clone(&shared);
        let progress = Arc::clone(&progress);
        handles.push(
            thread::Builder::new()
                .name(format!("betterkv-bench-{thread_index}"))
                .spawn(move || run_thread_shard(cfg, shard, progress))
                .map_err(|err| format!("failed to spawn benchmark thread {thread_index}: {err}"))?,
        );
    }

    let mut samples = Vec::new();
    let mut total_completed = 0u64;
    for handle in handles {
        let thread_stats = handle
            .join()
            .map_err(|_| "benchmark thread panicked".to_string())??;
        for stats in thread_stats {
            total_completed += stats.completed;
            samples.extend(stats.latencies_ns);
        }
    }

    progress.finished.store(true, Ordering::Relaxed);
    if let Some(reporter) = reporter {
        let _ = reporter.join();
        eprintln!();
    }

    let elapsed_secs = start.elapsed().as_secs_f64();
    if total_completed == 0 || elapsed_secs == 0.0 {
        return Err("benchmark completed with zero successful requests".to_string());
    }

    samples.sort_unstable();
    let avg_ms = samples.iter().copied().map(ns_to_ms).sum::<f64>() / samples.len() as f64;
    let min_ms = ns_to_ms(samples[0]);
    let p50_ms = percentile_ms(&samples, 50.0);
    let p95_ms = percentile_ms(&samples, 95.0);
    let p99_ms = percentile_ms(&samples, 99.0);
    let max_ms = ns_to_ms(*samples.last().unwrap_or(&0));

    Ok(BenchResult {
        name: shared.run.name.clone(),
        requests: total_completed,
        clients,
        elapsed_secs,
        req_per_sec: total_completed as f64 / elapsed_secs,
        avg_ms,
        min_ms,
        p50_ms,
        p95_ms,
        p99_ms,
        max_ms,
        data_size: shared.run.data_size,
        keep_alive: shared.run.keep_alive,
        multi_thread: args.multi_thread_enabled(),
        cumulative_distribution: build_cumulative_distribution(&samples),
        samples_ns: samples,
    })
}

pub async fn run_idle_mode(args: &Args) -> Result<(), String> {
    let addr = format!("{}:{}", args.host, args.port);
    let mut handles = Vec::with_capacity(args.clients);
    for _ in 0..args.clients {
        let addr = addr.clone();
        let user = args.user.clone();
        let password = args.password.clone();
        let dbnum = args.dbnum;
        handles.push(tokio::spawn(async move {
            let mut stream = TcpStream::connect(&addr)
                .await
                .map_err(|err| format!("connect {addr}: {err}"))?;
            stream
                .set_nodelay(true)
                .map_err(|err| format!("set_nodelay: {err}"))?;
            let mut parse_buf = BytesMut::with_capacity(256);
            authenticate_and_select_for_idle(
                &mut stream,
                &mut parse_buf,
                user.as_deref(),
                password.as_deref(),
                dbnum,
            )
            .await?;
            tokio::time::sleep(Duration::from_secs(u64::MAX / 4)).await;
            Ok::<(), String>(())
        }));
    }

    for handle in handles {
        handle
            .await
            .map_err(|err| format!("idle worker failed: {err}"))??;
    }

    Ok(())
}

fn spawn_progress_reporter(
    name: String,
    total: u64,
    progress: Arc<Progress>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let started = Instant::now();
        while !progress.finished.load(Ordering::Relaxed) {
            let completed = progress.completed.load(Ordering::Relaxed);
            eprint!(
                "\r{}",
                progress_line(&name, completed, total, started.elapsed().as_secs_f64())
            );
            thread::sleep(Duration::from_millis(200));
        }

        let completed = progress.completed.load(Ordering::Relaxed);
        eprint!(
            "\r{}",
            progress_line(&name, completed, total, started.elapsed().as_secs_f64())
        );
    })
}

fn run_thread_shard(
    cfg: Arc<Shared>,
    shard: Vec<ClientPlan>,
    progress: Arc<Progress>,
) -> Result<Vec<WorkerStats>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to create worker runtime: {err}"))?;

    runtime.block_on(async move {
        let mut handles = Vec::with_capacity(shard.len());
        for plan in shard {
            let worker_cfg = Arc::clone(&cfg);
            let worker_progress = Arc::clone(&progress);
            handles.push(tokio::spawn(async move {
                run_worker(plan.client_id, plan.quota, worker_cfg, worker_progress).await
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(
                handle
                    .await
                    .map_err(|err| format!("worker join error: {err}"))??,
            );
        }
        Ok(results)
    })
}

async fn run_worker(
    client_id: u64,
    quota: u64,
    cfg: Arc<Shared>,
    progress: Arc<Progress>,
) -> Result<WorkerStats, String> {
    let mut connection = open_connection(&cfg).await?;
    let value = vec![b'x'; cfg.run.data_size];
    let key_base = format!(
        "{}:{}:{client_id}",
        cfg.run.key_prefix,
        cfg.run.name.to_ascii_lowercase()
    );
    let mut random = RandomSource::new(cfg.run.seed ^ client_id.rotate_left(17));

    setup_connection_state(&mut connection, &cfg.run, key_base.as_bytes(), &value).await?;

    let mut stats = WorkerStats {
        latencies_ns: Vec::with_capacity(quota as usize),
        ..WorkerStats::default()
    };
    let mut remaining = quota;
    while remaining > 0 {
        let batch = remaining.min(cfg.run.pipeline as u64) as usize;
        if !cfg.run.keep_alive && stats.completed > 0 {
            connection = open_connection(&cfg).await?;
            setup_connection_state(&mut connection, &cfg.run, key_base.as_bytes(), &value).await?;
        }

        let request_group =
            build_request_group(&cfg.run, key_base.as_bytes(), &value, batch, &mut random)?;
        let sent_at = Instant::now();
        connection
            .stream
            .write_all(&request_group.payload)
            .await
            .map_err(|err| format!("write failed: {err}"))?;

        let mut pending = VecDeque::from(vec![sent_at; batch]);
        for index in 0..batch {
            let expected = request_group.expected[index].as_ref();
            let encoded = request_group.encoded[index].as_deref();
            consume_response(
                &mut connection.stream,
                &mut connection.parse_buf,
                expected,
                encoded,
                cfg.strict,
            )
            .await?;

            let started = pending.pop_front().expect("pending request timestamp");
            stats
                .latencies_ns
                .push(started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64);
        }

        stats.completed += batch as u64;
        progress
            .completed
            .fetch_add(batch as u64, Ordering::Relaxed);
        remaining -= batch as u64;
    }

    Ok(stats)
}

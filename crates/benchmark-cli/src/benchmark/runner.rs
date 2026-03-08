use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::cli::Args;
use crate::output::progress_line;
use crate::resp::consume_response;
use crate::workload::BenchRun;

use super::connection::{
    authenticate_and_select_for_idle, open_connection, setup_connection_state,
};
use super::model::{
    BenchResult, ClientPlan, Progress, RandomSource, RequestGroup, Shared, WorkerStats,
};
use super::request::build_request_group;
use super::stats::{build_cumulative_distribution, ns_to_ms, percentile_ms};

struct PreparedWork {
    quota: u64,
    connection: super::connection::Connection,
    reusable_full_batch: Option<RequestGroup>,
    reusable_tail_batch: Option<RequestGroup>,
    value: Vec<u8>,
    key_base: Vec<u8>,
    random: RandomSource,
}

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
        no_response_check: args.no_response_check,
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
    let active_threads = shards.iter().filter(|shard| !shard.is_empty()).count();
    let ready = Arc::new(Barrier::new(active_threads + 1));
    let start_gate = Arc::new(Barrier::new(active_threads + 1));

    let reporter = if args.quiet || args.csv {
        None
    } else {
        Some(spawn_progress_reporter(
            shared.run.name.clone(),
            shared.run.requests,
            Arc::clone(&progress),
        ))
    };

    let mut handles = Vec::with_capacity(thread_count);
    for (thread_index, shard) in shards.into_iter().enumerate() {
        if shard.is_empty() {
            continue;
        }

        let cfg = Arc::clone(&shared);
        let progress = Arc::clone(&progress);
        let ready = Arc::clone(&ready);
        let start_gate = Arc::clone(&start_gate);
        handles.push(
            thread::Builder::new()
                .name(format!("betterkv-bench-{thread_index}"))
                .spawn(move || run_thread_shard(cfg, shard, progress, ready, start_gate))
                .map_err(|err| format!("failed to spawn benchmark thread {thread_index}: {err}"))?,
        );
    }

    ready.wait();
    let start = Instant::now();
    start_gate.wait();

    let mut samples = Vec::new();
    let mut total_completed = 0u64;
    let mut total_build_ns = 0u64;
    let mut total_write_ns = 0u64;
    let mut total_response_ns = 0u64;
    for handle in handles {
        let thread_stats = handle
            .join()
            .map_err(|_| "benchmark thread panicked".to_string())??;
        for stats in thread_stats {
            total_completed += stats.completed;
            samples.extend(stats.latencies_ns);
            total_build_ns += stats.build_ns;
            total_write_ns += stats.write_ns;
            total_response_ns += stats.response_ns;
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
    let req_count = total_completed.max(1) as f64;
    let bench_build_ns_per_req = total_build_ns as f64 / req_count;
    let bench_write_ns_per_req = total_write_ns as f64 / req_count;
    let bench_response_ns_per_req = total_response_ns as f64 / req_count;
    let bench_total_ns_per_req =
        bench_build_ns_per_req + bench_write_ns_per_req + bench_response_ns_per_req;
    let avg_latency_ns = avg_ms * 1_000_000.0;
    let bench_pressure_pct = if avg_latency_ns > 0.0 {
        bench_total_ns_per_req * 100.0 / avg_latency_ns
    } else {
        0.0
    };

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
        bench_build_ns_per_req,
        bench_write_ns_per_req,
        bench_response_ns_per_req,
        bench_total_ns_per_req,
        bench_pressure_pct,
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
    ready: Arc<Barrier>,
    start_gate: Arc<Barrier>,
) -> Result<Vec<WorkerStats>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to create worker runtime: {err}"))?;

    runtime.block_on(async move {
        let mut handles = Vec::with_capacity(shard.len());
        for plan in shard {
            let worker_cfg = Arc::clone(&cfg);
            let prepared = prepare_worker(plan, &worker_cfg).await?;
            let worker_progress = Arc::clone(&progress);
            handles.push(tokio::spawn(async move {
                run_worker(prepared, worker_cfg, worker_progress).await
            }));
        }

        ready.wait();
        start_gate.wait();

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
    prepared: PreparedWork,
    cfg: Arc<Shared>,
    progress: Arc<Progress>,
) -> Result<WorkerStats, String> {
    let PreparedWork {
        quota,
        mut connection,
        reusable_full_batch,
        reusable_tail_batch,
        value,
        key_base,
        mut random,
    } = prepared;

    let mut stats = WorkerStats {
        latencies_ns: Vec::with_capacity(quota as usize),
        ..WorkerStats::default()
    };

    if cfg.run.keep_alive {
        if let Some(full_batch) = reusable_full_batch.clone() {
            return run_prebuilt_worker(
                quota,
                connection,
                full_batch,
                reusable_tail_batch.clone(),
                cfg,
                progress,
                stats,
            )
            .await;
        }
    }

    let mut remaining = quota;
    while remaining > 0 {
        let batch = remaining.min(cfg.run.pipeline as u64) as usize;
        if !cfg.run.keep_alive && stats.completed > 0 {
            connection = open_connection(&cfg).await?;
            setup_connection_state(&mut connection, &cfg.run, &key_base, &value).await?;
        }

        let dynamic_group;
        let request_group = match reusable_group(
            &reusable_full_batch,
            &reusable_tail_batch,
            batch,
            cfg.run.pipeline,
        ) {
            Some(group) => group,
            None => {
                let built_at = Instant::now();
                dynamic_group =
                    build_request_group(&cfg.run, &key_base, &value, batch, &mut random)?;
                stats.build_ns += built_at.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                &dynamic_group
            }
        };
        let sent_at = Instant::now();
        let write_started = Instant::now();
        connection
            .stream
            .write_all(&request_group.payload)
            .await
            .map_err(|err| format!("write failed: {err}"))?;
        stats.write_ns += write_started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;

        let response_started = Instant::now();
        if cfg.no_response_check {
            consume_responses_unchecked(
                &mut connection.stream,
                &mut connection.parse_buf,
                &request_group,
            )
            .await?;
        } else if request_group.uniform_encoded.is_some() {
            consume_uniform_responses(
                &mut connection.stream,
                &mut connection.parse_buf,
                &request_group,
                cfg.strict,
            )
            .await?;
        } else {
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
            stats.response_ns += response_started
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64;
            stats.completed += batch as u64;
            progress
                .completed
                .fetch_add(batch as u64, Ordering::Relaxed);
            remaining -= batch as u64;
            continue;
        }
        stats.response_ns += response_started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;

        record_batch(
            &mut stats,
            &progress,
            sent_at,
            batch,
            !cfg.no_response_check,
        );
        remaining -= batch as u64;
    }

    Ok(stats)
}

async fn prepare_worker(plan: ClientPlan, cfg: &Shared) -> Result<PreparedWork, String> {
    let value = vec![b'x'; cfg.run.data_size];
    let key_base = format!(
        "{}:{}:{}",
        cfg.run.key_prefix,
        cfg.run.name.to_ascii_lowercase(),
        plan.client_id
    )
    .into_bytes();
    let mut random = RandomSource::new(cfg.run.seed ^ plan.client_id.rotate_left(17));
    let mut connection = open_connection(cfg).await?;
    setup_connection_state(&mut connection, &cfg.run, &key_base, &value).await?;

    let reusable_full_batch = if can_reuse_request_group(&cfg.run) {
        Some(build_request_group(
            &cfg.run,
            &key_base,
            &value,
            cfg.run.pipeline,
            &mut random,
        )?)
    } else {
        None
    };
    let tail = (plan.quota % cfg.run.pipeline as u64) as usize;
    let reusable_tail_batch = if tail > 0 && can_reuse_request_group(&cfg.run) {
        Some(build_request_group(
            &cfg.run,
            &key_base,
            &value,
            tail,
            &mut random,
        )?)
    } else {
        None
    };

    Ok(PreparedWork {
        quota: plan.quota,
        connection,
        reusable_full_batch,
        reusable_tail_batch,
        value,
        key_base,
        random,
    })
}

fn can_reuse_request_group(run: &crate::workload::BenchRun) -> bool {
    if run.random_keyspace_len.unwrap_or(0) > 1 {
        return false;
    }

    match run.kind {
        crate::workload::BenchKind::Custom => run.command.as_ref().is_some_and(|command| {
            !command
                .parts
                .iter()
                .any(|part| matches!(part, crate::workload::ArgTemplate::RandomInt))
        }),
        _ => true,
    }
}

fn reusable_group<'a>(
    reusable_full_batch: &'a Option<RequestGroup>,
    reusable_tail_batch: &'a Option<RequestGroup>,
    batch: usize,
    pipeline: usize,
) -> Option<&'a RequestGroup> {
    if batch == pipeline {
        reusable_full_batch.as_ref()
    } else {
        reusable_tail_batch.as_ref()
    }
}

async fn consume_uniform_responses(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    request_group: &RequestGroup,
    strict: bool,
) -> Result<(), String> {
    let encoded = request_group
        .uniform_encoded
        .as_deref()
        .expect("uniform response bytes");
    crate::resp::consume_uniform_responses(
        stream,
        parse_buf,
        encoded,
        request_group.encoded.len(),
        strict,
    )
    .await
}

async fn consume_responses_unchecked(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    request_group: &RequestGroup,
) -> Result<(), String> {
    crate::resp::consume_responses_unchecked(stream, parse_buf, request_group).await
}

async fn run_prebuilt_worker(
    quota: u64,
    connection: super::connection::Connection,
    full_batch: RequestGroup,
    tail_batch: Option<RequestGroup>,
    cfg: Arc<Shared>,
    progress: Arc<Progress>,
    mut stats: WorkerStats,
) -> Result<WorkerStats, String> {
    let full_batch_count = quota / cfg.run.pipeline as u64;
    let tail_batch_len = (quota % cfg.run.pipeline as u64) as usize;
    let (mut reader, mut writer) = connection.stream.into_split();
    let mut parse_buf = connection.parse_buf;
    let (timestamps_tx, mut timestamps_rx) = mpsc::unbounded_channel::<(Instant, usize)>();
    let full_batch_len = full_batch.encoded.len();
    let full_batch_writer = full_batch.clone();
    let tail_batch_writer = tail_batch.clone();
    let write_ns = Arc::new(AtomicU64::new(0));
    let writer_write_ns = Arc::clone(&write_ns);

    let writer_task = tokio::spawn(async move {
        for _ in 0..full_batch_count {
            let sent_at = Instant::now();
            let write_started = Instant::now();
            writer
                .write_all(&full_batch_writer.payload)
                .await
                .map_err(|err| format!("write failed: {err}"))?;
            writer_write_ns.fetch_add(
                write_started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
                Ordering::Relaxed,
            );
            timestamps_tx
                .send((sent_at, full_batch_len))
                .map_err(|_| "writer timestamp channel closed".to_string())?;
        }

        if let Some(tail_batch) = tail_batch_writer {
            if tail_batch_len > 0 {
                let sent_at = Instant::now();
                let write_started = Instant::now();
                writer
                    .write_all(&tail_batch.payload)
                    .await
                    .map_err(|err| format!("write failed: {err}"))?;
                writer_write_ns.fetch_add(
                    write_started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
                    Ordering::Relaxed,
                );
                timestamps_tx
                    .send((sent_at, tail_batch.encoded.len()))
                    .map_err(|_| "writer timestamp channel closed".to_string())?;
            }
        }

        Ok::<(), String>(())
    });

    for _ in 0..full_batch_count {
        let response_started = Instant::now();
        consume_prebuilt_batch(
            &mut reader,
            &mut parse_buf,
            &full_batch,
            cfg.no_response_check,
            cfg.strict,
        )
        .await?;
        stats.response_ns += response_started
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        let (sent_at, batch) = timestamps_rx
            .recv()
            .await
            .ok_or_else(|| "writer timestamp channel closed".to_string())?;
        record_batch(
            &mut stats,
            &progress,
            sent_at,
            batch,
            !cfg.no_response_check,
        );
    }

    if let Some(tail_batch) = tail_batch.as_ref() {
        if tail_batch_len > 0 {
            let response_started = Instant::now();
            consume_prebuilt_batch(
                &mut reader,
                &mut parse_buf,
                tail_batch,
                cfg.no_response_check,
                cfg.strict,
            )
            .await?;
            stats.response_ns += response_started
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64;
            let (sent_at, batch) = timestamps_rx
                .recv()
                .await
                .ok_or_else(|| "writer timestamp channel closed".to_string())?;
            record_batch(
                &mut stats,
                &progress,
                sent_at,
                batch,
                !cfg.no_response_check,
            );
        }
    }

    writer_task
        .await
        .map_err(|err| format!("writer join error: {err}"))??;
    stats.write_ns += write_ns.load(Ordering::Relaxed);

    Ok(stats)
}

async fn consume_prebuilt_batch(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    parse_buf: &mut BytesMut,
    request_group: &RequestGroup,
    no_response_check: bool,
    strict: bool,
) -> Result<(), String> {
    if no_response_check {
        crate::resp::consume_responses_unchecked_read(reader, parse_buf, request_group).await
    } else if let Some(encoded) = request_group.uniform_encoded.as_deref() {
        crate::resp::consume_uniform_responses_read(
            reader,
            parse_buf,
            encoded,
            request_group.encoded.len(),
            strict,
        )
        .await
    } else {
        for index in 0..request_group.encoded.len() {
            crate::resp::consume_response_read(
                reader,
                parse_buf,
                request_group.expected[index].as_ref(),
                request_group.encoded[index].as_deref(),
                strict,
            )
            .await?;
        }
        Ok(())
    }
}

fn record_batch(
    stats: &mut WorkerStats,
    progress: &Progress,
    sent_at: Instant,
    batch: usize,
    repeat_each_request: bool,
) {
    let elapsed = sent_at.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
    if repeat_each_request {
        stats
            .latencies_ns
            .extend(std::iter::repeat_n(elapsed, batch));
    } else {
        stats.latencies_ns.push(elapsed);
    }
    stats.completed += batch as u64;
    progress
        .completed
        .fetch_add(batch as u64, Ordering::Relaxed);
}

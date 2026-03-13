use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use bytes::BytesMut;
use hdrhistogram::Histogram;
use indicatif::{ProgressBar, ProgressStyle};
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::task::JoinSet;

use crate::args::{Args, Connection, ConnectionTarget};
use crate::resp::{
    ExpectedResponse, append_resp_parts, encode_expected_response, encode_resp_parts,
    make_key_into, read_n_responses, read_n_strict_repeated_exact_responses,
    read_n_strict_responses, read_n_unchecked_repeated_exact_responses, read_n_unchecked_responses,
    repeat_payload,
};
use crate::spec::{BenchKind, BenchRun};

const SCRIPT_SET_BODY: &[u8] = b"redis.call('SET', KEYS[1], ARGV[1]); return ARGV[1]";
const SCRIPT_GET_BODY: &[u8] = b"return redis.call('GET', KEYS[1])";
const SETUP_BATCH: usize = 64;

struct WorkerStats {
    completed: u64,
    histogram: Histogram<u64>,
    total_latency_ns: u64,
}

impl WorkerStats {
    fn new() -> Result<Self, String> {
        Ok(Self {
            completed: 0,
            histogram: latency_histogram()?,
            total_latency_ns: 0,
        })
    }
}

pub struct BenchResult {
    pub name: String,
    pub requests: u64,
    pub warmup_requests: u64,
    pub clients: usize,
    pub elapsed_secs: f64,
    pub req_per_sec: f64,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub histogram: Histogram<u64>,
    pub data_size: usize,
    pub pipeline: usize,
    pub random_keys: bool,
    pub keyspace: u64,
}

#[derive(Clone, Copy)]
struct ProgressSnapshot {
    warming_up: bool,
    completed: u64,
    warmup_completed: u64,
    current_rps: f64,
    overall_rps: f64,
    current_avg_ms: f64,
    overall_avg_ms: f64,
    elapsed_secs: f64,
}

struct Shared {
    target: ConnectionTarget,
    user: Option<String>,
    password: Option<String>,
    strict: bool,
    spec: BenchRun,
    started: Instant,
    progress: Arc<ProgressState>,
}

struct ProgressState {
    completed: AtomicU64,
    warmup_completed: AtomicU64,
    latency_ns: AtomicU64,
    counted_start_ns: AtomicU64,
    stop: AtomicBool,
}

struct ResponseModel {
    kind: BenchKind,
    value: Vec<u8>,
    flags: Vec<bool>,
    ints: Vec<i64>,
    lens: Vec<i64>,
}

struct CommandScratch {
    key: Vec<u8>,
    related_key: Vec<u8>,
}

impl CommandScratch {
    fn new(base_len: usize) -> Self {
        Self {
            key: Vec::with_capacity(base_len + 21),
            related_key: Vec::with_capacity(base_len + 24),
        }
    }
}

impl ResponseModel {
    fn new(spec: &BenchRun, value: Vec<u8>) -> Self {
        let slots = if spec.random_keys {
            spec.keyspace as usize
        } else {
            1
        };

        let flags = match spec.kind {
            BenchKind::Lpop | BenchKind::Rpop | BenchKind::Srem | BenchKind::Zrem => {
                vec![true; slots]
            }
            _ => vec![false; slots],
        };

        let ints = vec![0; slots];
        let lens = vec![0; slots];

        Self {
            kind: spec.kind,
            value,
            flags,
            ints,
            lens,
        }
    }

    fn expected(&mut self, key_slot: u64) -> ExpectedResponse {
        let idx = (key_slot as usize).min(self.flags.len().saturating_sub(1));
        match self.kind {
            BenchKind::PingInline | BenchKind::PingMbulk => ExpectedResponse::Simple("PONG"),
            BenchKind::Echo
            | BenchKind::Get
            | BenchKind::GetSet
            | BenchKind::Hget
            | BenchKind::Eval
            | BenchKind::EvalRo
            | BenchKind::EvalSha
            | BenchKind::EvalShaRo => ExpectedResponse::Bulk(Some(self.value.clone())),
            BenchKind::Set | BenchKind::Mset => ExpectedResponse::Simple("OK"),
            BenchKind::Mget => ExpectedResponse::Array(vec![
                ExpectedResponse::Bulk(Some(self.value.clone())),
                ExpectedResponse::Bulk(Some(self.value.clone())),
            ]),
            BenchKind::SetNx | BenchKind::Sadd | BenchKind::Hset | BenchKind::Zadd => {
                let created = !self.flags[idx];
                self.flags[idx] = true;
                ExpectedResponse::Integer(created as i64)
            }
            BenchKind::Del => ExpectedResponse::Integer(0),
            BenchKind::Exists
            | BenchKind::Expire
            | BenchKind::Llen
            | BenchKind::Scard
            | BenchKind::Sismember
            | BenchKind::Zcard => ExpectedResponse::Integer(1),
            BenchKind::Ttl => ExpectedResponse::IntegerRange { min: 0, max: 60 },
            BenchKind::Incr => {
                self.ints[idx] += 1;
                ExpectedResponse::Integer(self.ints[idx])
            }
            BenchKind::IncrBy => {
                self.ints[idx] += 3;
                ExpectedResponse::Integer(self.ints[idx])
            }
            BenchKind::Decr => {
                self.ints[idx] -= 1;
                ExpectedResponse::Integer(self.ints[idx])
            }
            BenchKind::DecrBy => {
                self.ints[idx] -= 3;
                ExpectedResponse::Integer(self.ints[idx])
            }
            BenchKind::Strlen | BenchKind::SetRange => {
                ExpectedResponse::Integer(self.value.len() as i64)
            }
            BenchKind::GetRange => {
                ExpectedResponse::Bulk(Some(self.value[..self.value.len().min(3)].to_vec()))
            }
            BenchKind::Lpush | BenchKind::Rpush => {
                self.lens[idx] += 1;
                ExpectedResponse::Integer(self.lens[idx])
            }
            BenchKind::Lpop | BenchKind::Rpop => {
                let popped = self.flags[idx];
                self.flags[idx] = false;
                if popped {
                    ExpectedResponse::Bulk(Some(self.value.clone()))
                } else {
                    ExpectedResponse::Bulk(None)
                }
            }
            BenchKind::Lrange => {
                ExpectedResponse::Array(vec![ExpectedResponse::Bulk(Some(self.value.clone()))])
            }
            BenchKind::Srem | BenchKind::Zrem => {
                let removed = self.flags[idx];
                self.flags[idx] = false;
                ExpectedResponse::Integer(removed as i64)
            }
            BenchKind::Hgetall => ExpectedResponse::Array(vec![
                ExpectedResponse::Bulk(Some(b"field".to_vec())),
                ExpectedResponse::Bulk(Some(self.value.clone())),
            ]),
            BenchKind::Hincrby => {
                self.ints[idx] += 1;
                ExpectedResponse::Integer(self.ints[idx])
            }
            BenchKind::Zscore => ExpectedResponse::Bulk(Some(b"1".to_vec())),
            BenchKind::Zrank | BenchKind::Zrevrank => ExpectedResponse::Integer(0),
        }
    }
}

fn has_fixed_response_shape(kind: BenchKind) -> bool {
    matches!(
        kind,
        BenchKind::PingInline
            | BenchKind::PingMbulk
            | BenchKind::Echo
            | BenchKind::Set
            | BenchKind::Get
            | BenchKind::GetSet
            | BenchKind::Mset
            | BenchKind::Mget
            | BenchKind::Del
            | BenchKind::Exists
            | BenchKind::Expire
            | BenchKind::Strlen
            | BenchKind::SetRange
            | BenchKind::GetRange
            | BenchKind::Llen
            | BenchKind::Lrange
            | BenchKind::Scard
            | BenchKind::Sismember
            | BenchKind::Hget
            | BenchKind::Hgetall
            | BenchKind::Zcard
            | BenchKind::Zscore
            | BenchKind::Zrank
            | BenchKind::Zrevrank
            | BenchKind::Eval
            | BenchKind::EvalRo
            | BenchKind::EvalSha
            | BenchKind::EvalShaRo
    )
}

#[derive(Clone, Copy)]
struct ClientPlan {
    client_id: u64,
    warmup_quota: u64,
    tracked_quota: u64,
}

pub async fn run_single_benchmark(
    args: &Args,
    connection: &Connection,
    spec: BenchRun,
) -> Result<BenchResult, String> {
    let total_ops = spec.requests.saturating_add(spec.warmup_requests);
    let clients = spec.clients.min(total_ops.max(1) as usize).max(1);
    let tracked_base = spec.requests / clients as u64;
    let tracked_extra = (spec.requests % clients as u64) as usize;
    let warmup_base = spec.warmup_requests / clients as u64;
    let warmup_extra = (spec.warmup_requests % clients as u64) as usize;

    let progress = Arc::new(ProgressState {
        completed: AtomicU64::new(0),
        warmup_completed: AtomicU64::new(0),
        latency_ns: AtomicU64::new(0),
        counted_start_ns: AtomicU64::new(u64::MAX),
        stop: AtomicBool::new(false),
    });

    let start = Instant::now();

    let shared = Arc::new(Shared {
        target: connection.target.clone(),
        user: connection.user.clone(),
        password: connection.password.clone(),
        strict: args.strict,
        spec,
        started: start,
        progress: Arc::clone(&progress),
    });

    let mut plans = Vec::with_capacity(clients);
    for client_id in 0..clients {
        let tracked_quota = tracked_base + u64::from(client_id < tracked_extra);
        let warmup_quota = warmup_base + u64::from(client_id < warmup_extra);
        if tracked_quota == 0 && warmup_quota == 0 {
            continue;
        }
        plans.push(ClientPlan {
            client_id: client_id as u64,
            warmup_quota,
            tracked_quota,
        });
    }

    let progress_handle = if !args.quiet && !args.csv {
        let state = Arc::clone(&progress);
        let name = shared.spec.name.clone();
        let total = shared.spec.requests;
        Some(thread::spawn(move || {
            progress_loop(&name, total, state, start)
        }))
    } else {
        None
    };

    let mut worker_tasks = JoinSet::new();
    for plan in plans {
        let cfg = Arc::clone(&shared);
        worker_tasks.spawn(async move {
            run_worker(plan.client_id, plan.warmup_quota, plan.tracked_quota, cfg).await
        });
    }

    let mut total_completed = 0u64;
    let mut histogram = latency_histogram()?;
    let mut total_latency_ns = 0u64;
    while let Some(task_result) = worker_tasks.join_next().await {
        let stats = task_result.map_err(|err| format!("worker join error: {err}"))??;
        total_completed += stats.completed;
        total_latency_ns = total_latency_ns.saturating_add(stats.total_latency_ns);
        histogram
            .add(&stats.histogram)
            .map_err(|err| format!("failed to merge latency histogram: {err}"))?;
    }

    progress.stop.store(true, Ordering::Relaxed);
    if let Some(handle) = progress_handle {
        handle
            .join()
            .map_err(|_| "progress thread panicked".to_string())?;
        println!();
    }

    let elapsed = start.elapsed();
    let counted_start_ns = progress.counted_start_ns.load(Ordering::Relaxed);
    let elapsed_secs = if counted_start_ns == u64::MAX {
        elapsed.as_secs_f64()
    } else {
        (elapsed.as_nanos() as f64 - counted_start_ns as f64).max(0.0) / 1_000_000_000.0
    };
    if total_completed == 0 || elapsed_secs == 0.0 {
        return Err("benchmark completed with zero successful requests".to_string());
    }

    let avg_ms = total_latency_ns as f64 / total_completed as f64 / 1_000_000.0;
    let min_ms = histogram.min() as f64 / 1_000_000.0;
    let p50_ms = percentile_ms(&histogram, 0.50);
    let p95_ms = percentile_ms(&histogram, 0.95);
    let p99_ms = percentile_ms(&histogram, 0.99);
    let max_ms = histogram.max() as f64 / 1_000_000.0;

    Ok(BenchResult {
        name: shared.spec.name.clone(),
        requests: total_completed,
        warmup_requests: shared.spec.warmup_requests,
        clients,
        elapsed_secs,
        req_per_sec: total_completed as f64 / elapsed_secs,
        avg_ms,
        min_ms,
        p50_ms,
        p95_ms,
        p99_ms,
        max_ms,
        histogram,
        data_size: shared.spec.data_size,
        pipeline: shared.spec.pipeline,
        random_keys: shared.spec.random_keys,
        keyspace: shared.spec.keyspace,
    })
}

async fn run_worker(
    client_id: u64,
    mut warmup_remaining: u64,
    mut tracked_remaining: u64,
    cfg: Arc<Shared>,
) -> Result<WorkerStats, String> {
    let mut stream = connect_stream(&cfg.target).await?;

    let mut parse_buf = BytesMut::with_capacity(8192);

    if let Some(pass) = cfg.password.as_deref() {
        let auth = if let Some(user) = cfg.user.as_deref() {
            encode_resp_parts(&[b"AUTH", user.as_bytes(), pass.as_bytes()])
        } else {
            encode_resp_parts(&[b"AUTH", pass.as_bytes()])
        };
        stream
            .write_all(&auth)
            .await
            .map_err(|err| format!("AUTH write failed: {err}"))?;
        read_n_responses(&mut stream, &mut parse_buf, 1).await?;
    }

    let value = vec![b'x'; cfg.spec.data_size];
    let key_base = format!(
        "{}:{}:{client_id}",
        cfg.spec.key_prefix,
        cfg.spec
            .name
            .to_ascii_lowercase()
            .replace([' ', '/', '[', ']'], ":")
    );

    let script_sha = setup_worker_state(
        &mut stream,
        &mut parse_buf,
        &cfg.spec,
        key_base.as_bytes(),
        &value,
    )
    .await?;

    let mut stats = WorkerStats::new()?;
    let mut response_model = ResponseModel::new(&cfg.spec, value.clone());
    let mut sequence = 0u64;
    let fixed_exact_response = if has_fixed_response_shape(cfg.spec.kind) {
        let response = response_model.expected(0);
        encode_expected_response(&response)
    } else {
        None
    };
    let sample_command = build_command(
        cfg.spec.kind,
        key_base.as_bytes(),
        &value,
        0,
        script_sha.as_deref(),
    )?;

    if !cfg.spec.random_keys {
        let full_batch = repeat_payload(&sample_command, cfg.spec.pipeline);
        while warmup_remaining > 0 || tracked_remaining > 0 {
            let track = warmup_remaining == 0;
            let batch = if track {
                cfg.spec.pipeline
            } else {
                warmup_remaining.min(cfg.spec.pipeline as u64) as usize
            };
            if track {
                mark_counted_phase_started(&cfg.progress, cfg.started);
            }
            let started = Instant::now();
            if batch == cfg.spec.pipeline {
                stream
                    .write_all(&full_batch)
                    .await
                    .map_err(|err| format!("write failed: {err}"))?;
            } else {
                let tail = repeat_payload(&sample_command, batch);
                stream
                    .write_all(&tail)
                    .await
                    .map_err(|err| format!("write failed: {err}"))?;
            }
            let mut batch_latency_ns = 0u64;
            if let Some(encoded_response) = fixed_exact_response.as_deref() {
                if cfg.strict {
                    read_n_strict_repeated_exact_responses(
                        &mut stream,
                        &mut parse_buf,
                        encoded_response,
                        batch,
                        || {
                            if track {
                                let latency_ns =
                                    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                                record_latency_sample(&mut stats, latency_ns)?;
                                batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                            }
                            Ok(())
                        },
                    )
                    .await?;
                } else {
                    read_n_unchecked_repeated_exact_responses(
                        &mut stream,
                        &mut parse_buf,
                        encoded_response,
                        batch,
                        || {
                            if track {
                                let latency_ns =
                                    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                                record_latency_sample(&mut stats, latency_ns)?;
                                batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                            }
                            Ok(())
                        },
                    )
                    .await?;
                }
            } else {
                let expected = if cfg.strict {
                    (0..batch)
                        .map(|_| response_model.expected(0))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let encoded = if cfg.strict {
                    expected
                        .iter()
                        .map(encode_expected_response)
                        .collect::<Vec<_>>()
                } else {
                    (0..batch)
                        .map(|_| encode_expected_response(&response_model.expected(0)))
                        .collect::<Vec<_>>()
                };
                if cfg.strict {
                    read_n_strict_responses(
                        &mut stream,
                        &mut parse_buf,
                        &expected,
                        &encoded,
                        || {
                            if track {
                                let latency_ns =
                                    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                                record_latency_sample(&mut stats, latency_ns)?;
                                batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                            }
                            Ok(())
                        },
                    )
                    .await?;
                } else {
                    read_n_unchecked_responses(&mut stream, &mut parse_buf, &encoded, || {
                        if track {
                            let latency_ns =
                                started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                            record_latency_sample(&mut stats, latency_ns)?;
                            batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                        }
                        Ok(())
                    })
                    .await?;
                }
            }

            if track && batch_latency_ns > 0 {
                cfg.progress
                    .latency_ns
                    .fetch_add(batch_latency_ns, Ordering::Relaxed);
            }

            if track {
                let counted = tracked_remaining.min(batch as u64);
                stats.completed += counted;
                cfg.progress.completed.fetch_add(counted, Ordering::Relaxed);
                tracked_remaining = tracked_remaining.saturating_sub(counted);
            } else {
                cfg.progress
                    .warmup_completed
                    .fetch_add(batch as u64, Ordering::Relaxed);
                warmup_remaining = warmup_remaining.saturating_sub(batch as u64);
            }
        }
        return Ok(stats);
    }

    let mut payload = Vec::with_capacity(sample_command.len() * cfg.spec.pipeline.max(1));
    let mut command_scratch = CommandScratch::new(key_base.len());
    let mut expected = Vec::with_capacity(cfg.spec.pipeline);
    let mut encoded = Vec::with_capacity(cfg.spec.pipeline);

    while warmup_remaining > 0 || tracked_remaining > 0 {
        let track = warmup_remaining == 0;
        let batch = if track {
            cfg.spec.pipeline
        } else {
            warmup_remaining.min(cfg.spec.pipeline as u64) as usize
        };
        payload.clear();
        expected.clear();
        encoded.clear();
        for _ in 0..batch {
            let key_slot = random_slot(client_id, sequence, cfg.spec.keyspace);
            append_command(
                &mut payload,
                cfg.spec.kind,
                key_base.as_bytes(),
                &value,
                key_slot,
                script_sha.as_deref(),
                &mut command_scratch,
            )?;
            if fixed_exact_response.is_none() {
                let response = response_model.expected(key_slot);
                encoded.push(encode_expected_response(&response));
                if cfg.strict {
                    expected.push(response);
                }
            }
            sequence = sequence.wrapping_add(1);
        }

        if track {
            mark_counted_phase_started(&cfg.progress, cfg.started);
        }
        let started = Instant::now();
        stream
            .write_all(&payload)
            .await
            .map_err(|err| format!("write failed: {err}"))?;
        let mut batch_latency_ns = 0u64;
        if let Some(encoded_response) = fixed_exact_response.as_deref() {
            if cfg.strict {
                read_n_strict_repeated_exact_responses(
                    &mut stream,
                    &mut parse_buf,
                    encoded_response,
                    batch,
                    || {
                        if track {
                            let latency_ns =
                                started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                            record_latency_sample(&mut stats, latency_ns)?;
                            batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                        }
                        Ok(())
                    },
                )
                .await?;
            } else {
                read_n_unchecked_repeated_exact_responses(
                    &mut stream,
                    &mut parse_buf,
                    encoded_response,
                    batch,
                    || {
                        if track {
                            let latency_ns =
                                started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                            record_latency_sample(&mut stats, latency_ns)?;
                            batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                        }
                        Ok(())
                    },
                )
                .await?;
            }
        } else {
            if cfg.strict {
                read_n_strict_responses(&mut stream, &mut parse_buf, &expected, &encoded, || {
                    if track {
                        let latency_ns =
                            started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                        record_latency_sample(&mut stats, latency_ns)?;
                        batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                    }
                    Ok(())
                })
                .await?;
            } else {
                read_n_unchecked_responses(&mut stream, &mut parse_buf, &encoded, || {
                    if track {
                        let latency_ns =
                            started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
                        record_latency_sample(&mut stats, latency_ns)?;
                        batch_latency_ns = batch_latency_ns.saturating_add(latency_ns);
                    }
                    Ok(())
                })
                .await?;
            }
        }

        if track && batch_latency_ns > 0 {
            cfg.progress
                .latency_ns
                .fetch_add(batch_latency_ns, Ordering::Relaxed);
        }

        if track {
            let counted = tracked_remaining.min(batch as u64);
            stats.completed += counted;
            cfg.progress.completed.fetch_add(counted, Ordering::Relaxed);
            tracked_remaining = tracked_remaining.saturating_sub(counted);
        } else {
            cfg.progress
                .warmup_completed
                .fetch_add(batch as u64, Ordering::Relaxed);
            warmup_remaining = warmup_remaining.saturating_sub(batch as u64);
        }
    }

    Ok(stats)
}

fn progress_loop(name: &str, total_requests: u64, state: Arc<ProgressState>, started: Instant) {
    let mut last_count = 0u64;
    let mut last_latency = 0u64;
    let mut last_tick = started;
    let progress = ProgressBar::new(total_requests);
    progress.enable_steady_tick(Duration::from_millis(120));
    let style = match ProgressStyle::with_template(
        "{spinner:.cyan} {msg} [{wide_bar:.cyan/blue}] {pos}/{len} {percent:>3}% | {per_sec} | avg {eta_precise}",
    ) {
        Ok(style) => style.progress_chars("=> "),
        Err(_) => ProgressStyle::default_bar(),
    };
    progress.set_style(style);

    loop {
        thread::sleep(Duration::from_millis(250));

        let snapshot = progress_snapshot(&state, started, last_tick, last_count, last_latency);
        last_count = if snapshot.warming_up {
            snapshot.warmup_completed
        } else {
            snapshot.completed
        };
        last_latency = state.latency_ns.load(Ordering::Relaxed);
        last_tick = Instant::now();

        progress.set_position(snapshot.completed.min(total_requests));
        progress.set_message(format!(
            "{} | {} | live {} | overall {} | lat {} / {} | elapsed {}",
            name,
            if snapshot.warming_up {
                "warmup"
            } else {
                "measured"
            },
            format_rps(snapshot.current_rps),
            format_rps(snapshot.overall_rps),
            format_latency_brief(snapshot.current_avg_ms),
            format_latency_brief(snapshot.overall_avg_ms),
            format_duration(snapshot.elapsed_secs),
        ));

        if state.stop.load(Ordering::Relaxed) {
            progress.finish_and_clear();
            break;
        }
    }
}

fn progress_snapshot(
    state: &ProgressState,
    started: Instant,
    last_tick: Instant,
    last_count: u64,
    last_latency: u64,
) -> ProgressSnapshot {
    let completed = state.completed.load(Ordering::Relaxed);
    let warmup_completed = state.warmup_completed.load(Ordering::Relaxed);
    let total_latency = state.latency_ns.load(Ordering::Relaxed);
    let counted_start_ns = state.counted_start_ns.load(Ordering::Relaxed);
    let now = Instant::now();
    let dt = now.duration_since(last_tick).as_secs_f64().max(0.000_001);
    let total_elapsed_secs = now.duration_since(started).as_secs_f64().max(0.000_001);
    let warming_up = counted_start_ns == u64::MAX;
    let elapsed_secs = if warming_up {
        total_elapsed_secs
    } else {
        (now.duration_since(started).as_nanos() as f64 - counted_start_ns as f64).max(0.0)
            / 1_000_000_000.0
    }
    .max(0.000_001);
    let active_completed = if warming_up {
        warmup_completed
    } else {
        completed
    };
    let delta = active_completed.saturating_sub(last_count);
    let delta_latency = total_latency.saturating_sub(last_latency);

    ProgressSnapshot {
        warming_up,
        completed,
        warmup_completed,
        current_rps: delta as f64 / dt,
        overall_rps: active_completed as f64 / elapsed_secs,
        current_avg_ms: if delta == 0 {
            0.0
        } else {
            delta_latency as f64 / delta as f64 / 1_000_000.0
        },
        overall_avg_ms: if completed == 0 {
            0.0
        } else {
            total_latency as f64 / completed as f64 / 1_000_000.0
        },
        elapsed_secs,
    }
}

fn format_rps(rps: f64) -> String {
    if rps >= 1_000_000.0 {
        format!("{:.2}M rps", rps / 1_000_000.0)
    } else if rps >= 1_000.0 {
        format!("{:.2}K rps", rps / 1_000.0)
    } else {
        format!("{rps:.0} rps")
    }
}

fn format_latency_brief(ms: f64) -> String {
    if ms >= 1.0 {
        format!("{ms:.3} ms")
    } else if ms >= 0.001 {
        format!("{:.2} us", ms * 1_000.0)
    } else {
        format!("{:.0} ns", ms * 1_000_000.0)
    }
}

fn format_duration(seconds: f64) -> String {
    if seconds >= 60.0 {
        format!("{:.1}m", seconds / 60.0)
    } else {
        format!("{seconds:.1}s")
    }
}

async fn setup_worker_state(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    parse_buf: &mut BytesMut,
    spec: &BenchRun,
    key_base: &[u8],
    value: &[u8],
) -> Result<Option<Vec<u8>>, String> {
    if spec.random_keys {
        prime_keyspace(stream, parse_buf, spec.kind, key_base, value, spec.keyspace).await?;
    } else if let Some(setup) = build_setup_command(spec.kind, key_base, value) {
        stream
            .write_all(&setup)
            .await
            .map_err(|err| format!("setup write failed: {err}"))?;
        read_n_responses(stream, parse_buf, 1).await?;
    }

    let script = match spec.kind {
        BenchKind::EvalSha => Some(SCRIPT_SET_BODY),
        BenchKind::EvalShaRo => Some(SCRIPT_GET_BODY),
        _ => None,
    };

    let Some(script) = script else {
        return Ok(None);
    };

    let load = encode_resp_parts(&[b"SCRIPT", b"LOAD", script]);
    stream
        .write_all(&load)
        .await
        .map_err(|err| format!("script load write failed: {err}"))?;
    let frame = read_one_response(stream, parse_buf).await?;
    match frame {
        RespFrame::Bulk(Some(BulkData::Arg(value))) => Ok(Some(value.to_vec())),
        RespFrame::Bulk(Some(BulkData::Value(value))) => Ok(Some(value.to_vec())),
        RespFrame::Error(message) => Err(format!("script load failed: {message}")),
        RespFrame::ErrorStatic(message) => Err(format!("script load failed: {message}")),
        other => Err(format!("unexpected SCRIPT LOAD response: {other:?}")),
    }
}

async fn prime_keyspace(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    parse_buf: &mut BytesMut,
    kind: BenchKind,
    key_base: &[u8],
    value: &[u8],
    keyspace: u64,
) -> Result<(), String> {
    if !requires_existing_state(kind) {
        return Ok(());
    }

    let mut payload = Vec::new();
    let mut command_scratch = CommandScratch::new(key_base.len());
    let mut pending = 0usize;
    for slot in 0..keyspace {
        make_key_into(key_base, slot, &mut command_scratch.key);
        if append_setup_command(
            &mut payload,
            kind,
            command_scratch.key.as_slice(),
            value,
            &mut command_scratch.related_key,
        ) {
            pending += 1;
        }

        if pending == SETUP_BATCH {
            stream
                .write_all(&payload)
                .await
                .map_err(|err| format!("setup write failed: {err}"))?;
            read_n_responses(stream, parse_buf, pending).await?;
            payload.clear();
            pending = 0;
        }
    }

    if pending > 0 {
        stream
            .write_all(&payload)
            .await
            .map_err(|err| format!("setup write failed: {err}"))?;
        read_n_responses(stream, parse_buf, pending).await?;
    }

    Ok(())
}

async fn read_one_response(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
) -> Result<RespFrame, String> {
    let mut chunk = [0u8; 8192];
    loop {
        match parser::parse_frame(parse_buf) {
            Ok(Some(frame)) => return Ok(frame),
            Ok(None) | Err(ParseError::Incomplete) => {}
            Err(ParseError::Protocol(err)) => return Err(format!("protocol error: {err}")),
        }

        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
        parse_buf.extend_from_slice(&chunk[..read]);
    }
}

async fn connect_stream(target: &ConnectionTarget) -> Result<Box<dyn AsyncReadWrite>, String> {
    match target {
        ConnectionTarget::Tcp { host, port } => {
            let addr = format!("{host}:{port}");
            let stream = TcpStream::connect(&addr)
                .await
                .map_err(|err| format!("connect {addr}: {err}"))?;
            stream
                .set_nodelay(true)
                .map_err(|err| format!("set_nodelay: {err}"))?;
            Ok(Box::new(stream) as Box<dyn AsyncReadWrite>)
        }
        ConnectionTarget::Unix { path } => {
            let stream = UnixStream::connect(path)
                .await
                .map_err(|err| format!("connect unix:{path}: {err}"))?;
            Ok(Box::new(stream) as Box<dyn AsyncReadWrite>)
        }
    }
}

trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> AsyncReadWrite for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

fn build_setup_command(kind: BenchKind, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(value.len() + key.len() + 64);
    let mut related_key = Vec::with_capacity(key.len() + 3);
    append_setup_command(&mut out, kind, key, value, &mut related_key).then_some(out)
}

fn append_setup_command(
    out: &mut Vec<u8>,
    kind: BenchKind,
    key: &[u8],
    value: &[u8],
    related_key: &mut Vec<u8>,
) -> bool {
    match kind {
        BenchKind::Get
        | BenchKind::GetSet
        | BenchKind::Exists
        | BenchKind::Expire
        | BenchKind::Ttl
        | BenchKind::Strlen
        | BenchKind::SetRange
        | BenchKind::GetRange
        | BenchKind::EvalRo
        | BenchKind::EvalShaRo => {
            append_resp_parts(out, &[b"SET", key, value]);
            true
        }
        BenchKind::Mget => {
            related_multi_key_into(key, related_key);
            append_resp_parts(out, &[b"MSET", key, value, related_key.as_slice(), value]);
            true
        }
        BenchKind::Mset => {
            related_multi_key_into(key, related_key);
            append_resp_parts(out, &[b"DEL", key, related_key.as_slice()]);
            true
        }
        BenchKind::Lpop | BenchKind::Rpop | BenchKind::Llen | BenchKind::Lrange => {
            append_resp_parts(out, &[b"LPUSH", key, value]);
            true
        }
        BenchKind::Srem | BenchKind::Scard | BenchKind::Sismember => {
            append_resp_parts(out, &[b"SADD", key, value]);
            true
        }
        BenchKind::Hget | BenchKind::Hgetall => {
            append_resp_parts(out, &[b"HSET", key, b"field", value]);
            true
        }
        BenchKind::Hincrby => {
            append_resp_parts(out, &[b"HSET", key, b"field", b"0"]);
            true
        }
        BenchKind::Zrem
        | BenchKind::Zcard
        | BenchKind::Zscore
        | BenchKind::Zrank
        | BenchKind::Zrevrank => {
            append_resp_parts(out, &[b"ZADD", key, b"1", value]);
            true
        }
        _ => false,
    }
}

fn requires_existing_state(kind: BenchKind) -> bool {
    matches!(
        kind,
        BenchKind::Get
            | BenchKind::GetSet
            | BenchKind::Exists
            | BenchKind::Expire
            | BenchKind::Ttl
            | BenchKind::Strlen
            | BenchKind::SetRange
            | BenchKind::GetRange
            | BenchKind::EvalRo
            | BenchKind::EvalShaRo
            | BenchKind::Mget
            | BenchKind::Lpop
            | BenchKind::Rpop
            | BenchKind::Llen
            | BenchKind::Lrange
            | BenchKind::Srem
            | BenchKind::Scard
            | BenchKind::Sismember
            | BenchKind::Hget
            | BenchKind::Hgetall
            | BenchKind::Hincrby
            | BenchKind::Zrem
            | BenchKind::Zcard
            | BenchKind::Zscore
            | BenchKind::Zrank
            | BenchKind::Zrevrank
    )
}

fn related_multi_key_into(key: &[u8], related: &mut Vec<u8>) {
    related.clear();
    related.extend_from_slice(key);
    related.extend_from_slice(b":m2");
}

fn append_command(
    out: &mut Vec<u8>,
    kind: BenchKind,
    key_base: &[u8],
    value: &[u8],
    key_slot: u64,
    script_sha: Option<&[u8]>,
    scratch: &mut CommandScratch,
) -> Result<(), String> {
    match kind {
        BenchKind::PingInline => out.extend_from_slice(b"PING\r\n"),
        BenchKind::PingMbulk => append_resp_parts(out, &[b"PING"]),
        BenchKind::Echo => append_resp_parts(out, &[b"ECHO", value]),
        BenchKind::Set => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SET", scratch.key.as_slice(), value]);
        }
        BenchKind::SetNx => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SETNX", scratch.key.as_slice(), value]);
        }
        BenchKind::Get => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"GET", scratch.key.as_slice()]);
        }
        BenchKind::GetSet => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"GETSET", scratch.key.as_slice(), value]);
        }
        BenchKind::Mset => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            related_multi_key_into(scratch.key.as_slice(), &mut scratch.related_key);
            append_resp_parts(
                out,
                &[
                    b"MSET",
                    scratch.key.as_slice(),
                    value,
                    scratch.related_key.as_slice(),
                    value,
                ],
            );
        }
        BenchKind::Mget => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            related_multi_key_into(scratch.key.as_slice(), &mut scratch.related_key);
            append_resp_parts(
                out,
                &[
                    b"MGET",
                    scratch.key.as_slice(),
                    scratch.related_key.as_slice(),
                ],
            );
        }
        BenchKind::Del => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"DEL", scratch.key.as_slice()]);
        }
        BenchKind::Exists => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"EXISTS", scratch.key.as_slice()]);
        }
        BenchKind::Expire => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"EXPIRE", scratch.key.as_slice(), b"60"]);
        }
        BenchKind::Ttl => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"TTL", scratch.key.as_slice()]);
        }
        BenchKind::Incr => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"INCR", scratch.key.as_slice()]);
        }
        BenchKind::IncrBy => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"INCRBY", scratch.key.as_slice(), b"3"]);
        }
        BenchKind::Decr => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"DECR", scratch.key.as_slice()]);
        }
        BenchKind::DecrBy => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"DECRBY", scratch.key.as_slice(), b"3"]);
        }
        BenchKind::Strlen => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"STRLEN", scratch.key.as_slice()]);
        }
        BenchKind::SetRange => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SETRANGE", scratch.key.as_slice(), b"0", value]);
        }
        BenchKind::GetRange => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"GETRANGE", scratch.key.as_slice(), b"0", b"2"]);
        }
        BenchKind::Lpush => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"LPUSH", scratch.key.as_slice(), value]);
        }
        BenchKind::Rpush => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"RPUSH", scratch.key.as_slice(), value]);
        }
        BenchKind::Lpop => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"LPOP", scratch.key.as_slice()]);
        }
        BenchKind::Rpop => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"RPOP", scratch.key.as_slice()]);
        }
        BenchKind::Llen => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"LLEN", scratch.key.as_slice()]);
        }
        BenchKind::Lrange => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"LRANGE", scratch.key.as_slice(), b"0", b"9"]);
        }
        BenchKind::Sadd => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SADD", scratch.key.as_slice(), value]);
        }
        BenchKind::Srem => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SREM", scratch.key.as_slice(), value]);
        }
        BenchKind::Scard => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SCARD", scratch.key.as_slice()]);
        }
        BenchKind::Sismember => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"SISMEMBER", scratch.key.as_slice(), value]);
        }
        BenchKind::Hset => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"HSET", scratch.key.as_slice(), b"field", value]);
        }
        BenchKind::Hget => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"HGET", scratch.key.as_slice(), b"field"]);
        }
        BenchKind::Hgetall => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"HGETALL", scratch.key.as_slice()]);
        }
        BenchKind::Hincrby => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"HINCRBY", scratch.key.as_slice(), b"field", b"1"]);
        }
        BenchKind::Zadd => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZADD", scratch.key.as_slice(), b"1", value]);
        }
        BenchKind::Zrem => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZREM", scratch.key.as_slice(), value]);
        }
        BenchKind::Zcard => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZCARD", scratch.key.as_slice()]);
        }
        BenchKind::Zscore => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZSCORE", scratch.key.as_slice(), value]);
        }
        BenchKind::Zrank => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZRANK", scratch.key.as_slice(), value]);
        }
        BenchKind::Zrevrank => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(out, &[b"ZREVRANK", scratch.key.as_slice(), value]);
        }
        BenchKind::Eval => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(
                out,
                &[
                    b"EVAL",
                    SCRIPT_SET_BODY,
                    b"1",
                    scratch.key.as_slice(),
                    value,
                ],
            );
        }
        BenchKind::EvalRo => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            append_resp_parts(
                out,
                &[b"EVAL_RO", SCRIPT_GET_BODY, b"1", scratch.key.as_slice()],
            );
        }
        BenchKind::EvalSha => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            let sha =
                script_sha.ok_or_else(|| "missing script sha for EVALSHA benchmark".to_string())?;
            append_resp_parts(out, &[b"EVALSHA", sha, b"1", scratch.key.as_slice(), value]);
        }
        BenchKind::EvalShaRo => {
            make_key_into(key_base, key_slot, &mut scratch.key);
            let sha = script_sha
                .ok_or_else(|| "missing script sha for EVALSHA_RO benchmark".to_string())?;
            append_resp_parts(out, &[b"EVALSHA_RO", sha, b"1", scratch.key.as_slice()]);
        }
    }

    Ok(())
}

fn build_command(
    kind: BenchKind,
    key_base: &[u8],
    value: &[u8],
    key_slot: u64,
    script_sha: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(value.len() + key_base.len() + 96);
    let mut scratch = CommandScratch::new(key_base.len());
    append_command(
        &mut out,
        kind,
        key_base,
        value,
        key_slot,
        script_sha,
        &mut scratch,
    )?;
    Ok(out)
}

fn random_slot(client_id: u64, sequence: u64, keyspace: u64) -> u64 {
    if keyspace <= 1 {
        return 0;
    }

    splitmix64((client_id << 32) ^ sequence) % keyspace
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn percentile_ms(histogram: &Histogram<u64>, percentile: f64) -> f64 {
    if histogram.is_empty() {
        return 0.0;
    }

    histogram.value_at_quantile(percentile) as f64 / 1_000_000.0
}

fn latency_histogram() -> Result<Histogram<u64>, String> {
    Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)
        .map_err(|err| format!("failed to create latency histogram: {err}"))
}

fn record_latency_sample(stats: &mut WorkerStats, latency_ns: u64) -> Result<(), String> {
    stats
        .histogram
        .record(latency_ns.max(1))
        .map_err(|err| format!("failed to record latency: {err}"))?;
    stats.total_latency_ns = stats.total_latency_ns.saturating_add(latency_ns);
    Ok(())
}

fn mark_counted_phase_started(progress: &ProgressState, started: Instant) {
    let counted_start_ns = started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
    let _ = progress.counted_start_ns.compare_exchange(
        u64::MAX,
        counted_start_ns,
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
}

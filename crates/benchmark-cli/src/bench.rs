use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::args::Args;
use crate::resp::{encode_resp_parts, make_key, read_n_responses, repeat_payload};
use crate::spec::{BenchKind, BenchSpec};

#[derive(Default)]
struct WorkerStats {
    completed: u64,
    lat_samples_ns: Vec<u64>,
}

pub struct BenchResult {
    pub name: &'static str,
    pub requests: u64,
    pub clients: usize,
    pub elapsed_secs: f64,
    pub req_per_sec: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

struct Shared {
    host: String,
    port: u16,
    pipeline: usize,
    data_size: usize,
    random_keys: bool,
    auth: Option<String>,
    spec: BenchSpec,
}

pub async fn run_single_benchmark(args: &Args, spec: BenchSpec) -> Result<BenchResult, String> {
    let clients = args.clients.min(args.requests as usize).max(1);
    let base = args.requests / clients as u64;
    let extra = (args.requests % clients as u64) as usize;

    let shared = Arc::new(Shared {
        host: args.host.clone(),
        port: args.port,
        pipeline: args.pipeline,
        data_size: args.data_size,
        random_keys: args.random_keys,
        auth: args.auth.clone(),
        spec,
    });

    let mut handles = Vec::with_capacity(clients);
    let start = Instant::now();
    for client_id in 0..clients {
        let quota = base + u64::from(client_id < extra);
        if quota == 0 {
            continue;
        }
        let cfg = Arc::clone(&shared);
        handles.push(tokio::spawn(async move {
            run_worker(client_id as u64, quota, cfg).await
        }));
    }

    let mut total_completed = 0u64;
    let mut samples = Vec::<u64>::new();
    for handle in handles {
        let stats = handle
            .await
            .map_err(|err| format!("worker join error: {err}"))??;
        total_completed += stats.completed;
        samples.extend(stats.lat_samples_ns);
    }

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    if total_completed == 0 || elapsed_secs == 0.0 {
        return Err("benchmark completed with zero successful requests".to_string());
    }

    samples.sort_unstable();
    let avg_ms = elapsed_secs * 1000.0 / total_completed as f64;
    let p50_ms = percentile_ms(&samples, 0.50);
    let p95_ms = percentile_ms(&samples, 0.95);
    let p99_ms = percentile_ms(&samples, 0.99);

    Ok(BenchResult {
        name: spec.name,
        requests: total_completed,
        clients,
        elapsed_secs,
        req_per_sec: total_completed as f64 / elapsed_secs,
        avg_ms,
        p50_ms,
        p95_ms,
        p99_ms,
    })
}

async fn run_worker(client_id: u64, quota: u64, cfg: Arc<Shared>) -> Result<WorkerStats, String> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let mut stream = TcpStream::connect(&addr)
        .await
        .map_err(|err| format!("connect {addr}: {err}"))?;
    stream
        .set_nodelay(true)
        .map_err(|err| format!("set_nodelay: {err}"))?;

    let mut parse_buf = BytesMut::with_capacity(8192);

    if let Some(pass) = cfg.auth.as_deref() {
        let auth = encode_resp_parts(&[b"AUTH", pass.as_bytes()]);
        stream
            .write_all(&auth)
            .await
            .map_err(|err| format!("AUTH write failed: {err}"))?;
        read_n_responses(&mut stream, &mut parse_buf, 1).await?;
    }

    let value = vec![b'x'; cfg.data_size];
    let key_fixed = format!(
        "justkv:bench:{}:{client_id}",
        cfg.spec.name.to_ascii_lowercase()
    );

    if let Some(setup) = build_setup_command(cfg.spec.kind, key_fixed.as_bytes(), &value) {
        stream
            .write_all(&setup)
            .await
            .map_err(|err| format!("setup write failed: {err}"))?;
        read_n_responses(&mut stream, &mut parse_buf, 1).await?;
    }

    let mut stats = WorkerStats::default();
    let mut sequence = 0u64;

    if !cfg.random_keys {
        let one = build_command(cfg.spec.kind, key_fixed.as_bytes(), &value, 0);
        let full_batch = repeat_payload(&one, cfg.pipeline);
        let mut remaining = quota;

        while remaining > 0 {
            let batch = remaining.min(cfg.pipeline as u64) as usize;
            let started = Instant::now();
            if batch == cfg.pipeline {
                stream
                    .write_all(&full_batch)
                    .await
                    .map_err(|err| format!("write failed: {err}"))?;
            } else {
                let tail = repeat_payload(&one, batch);
                stream
                    .write_all(&tail)
                    .await
                    .map_err(|err| format!("write failed: {err}"))?;
            }
            read_n_responses(&mut stream, &mut parse_buf, batch).await?;

            let per_req_ns =
                (started.elapsed().as_nanos() / batch as u128).min(u128::from(u64::MAX));
            stats.lat_samples_ns.push(per_req_ns as u64);
            stats.completed += batch as u64;
            remaining -= batch as u64;
        }
        return Ok(stats);
    }

    let mut remaining = quota;
    while remaining > 0 {
        let batch = remaining.min(cfg.pipeline as u64) as usize;
        let mut payload = Vec::with_capacity(batch * (cfg.data_size + 96));
        for _ in 0..batch {
            let command = build_command(cfg.spec.kind, key_fixed.as_bytes(), &value, sequence);
            payload.extend_from_slice(&command);
            sequence = sequence.wrapping_add(1);
        }

        let started = Instant::now();
        stream
            .write_all(&payload)
            .await
            .map_err(|err| format!("write failed: {err}"))?;
        read_n_responses(&mut stream, &mut parse_buf, batch).await?;

        let per_req_ns = (started.elapsed().as_nanos() / batch as u128).min(u128::from(u64::MAX));
        stats.lat_samples_ns.push(per_req_ns as u64);
        stats.completed += batch as u64;
        remaining -= batch as u64;
    }

    Ok(stats)
}

fn build_setup_command(kind: BenchKind, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
    match kind {
        BenchKind::Get
        | BenchKind::GetSet
        | BenchKind::Mget
        | BenchKind::Exists
        | BenchKind::Expire
        | BenchKind::Ttl
        | BenchKind::Strlen
        | BenchKind::SetRange
        | BenchKind::GetRange => Some(encode_resp_parts(&[b"SET", key, value])),
        BenchKind::Mset => Some(encode_resp_parts(&[b"DEL", key, b"bench:m2"])),
        BenchKind::Lpop | BenchKind::Rpop | BenchKind::Llen | BenchKind::Lrange => {
            Some(encode_resp_parts(&[b"LPUSH", key, value]))
        }
        BenchKind::Srem | BenchKind::Scard | BenchKind::Sismember => {
            Some(encode_resp_parts(&[b"SADD", key, value]))
        }
        BenchKind::Hget | BenchKind::Hgetall => Some(encode_resp_parts(&[b"HSET", key, b"field", value])),
        BenchKind::Hincrby => Some(encode_resp_parts(&[b"HSET", key, b"field", b"0"])),
        BenchKind::Zrem
        | BenchKind::Zcard
        | BenchKind::Zscore
        | BenchKind::Zrank
        | BenchKind::Zrevrank => Some(encode_resp_parts(&[b"ZADD", key, b"1", value])),
        _ => None,
    }
}

fn build_command(kind: BenchKind, key_base: &[u8], value: &[u8], sequence: u64) -> Vec<u8> {
    match kind {
        BenchKind::PingInline => b"PING\r\n".to_vec(),
        BenchKind::PingMbulk => encode_resp_parts(&[b"PING"]),
        BenchKind::Echo => encode_resp_parts(&[b"ECHO", value]),
        BenchKind::Set => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SET", key.as_slice(), value])
        }
        BenchKind::SetNx => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SETNX", key.as_slice(), value])
        }
        BenchKind::Get => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"GET", key.as_slice()])
        }
        BenchKind::GetSet => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"GETSET", key.as_slice(), value])
        }
        BenchKind::Mset => {
            let key1 = make_key(key_base, sequence);
            let mut key2 = key1.clone();
            key2.extend_from_slice(b":m2");
            encode_resp_parts(&[b"MSET", key1.as_slice(), value, key2.as_slice(), value])
        }
        BenchKind::Mget => {
            let key1 = make_key(key_base, sequence);
            let mut key2 = key1.clone();
            key2.extend_from_slice(b":m2");
            encode_resp_parts(&[b"MGET", key1.as_slice(), key2.as_slice()])
        }
        BenchKind::Del => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"DEL", key.as_slice()])
        }
        BenchKind::Exists => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"EXISTS", key.as_slice()])
        }
        BenchKind::Expire => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"EXPIRE", key.as_slice(), b"60"])
        }
        BenchKind::Ttl => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"TTL", key.as_slice()])
        }
        BenchKind::Incr => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"INCR", key.as_slice()])
        }
        BenchKind::IncrBy => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"INCRBY", key.as_slice(), b"3"])
        }
        BenchKind::Decr => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"DECR", key.as_slice()])
        }
        BenchKind::DecrBy => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"DECRBY", key.as_slice(), b"3"])
        }
        BenchKind::Strlen => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"STRLEN", key.as_slice()])
        }
        BenchKind::SetRange => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SETRANGE", key.as_slice(), b"0", value])
        }
        BenchKind::GetRange => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"GETRANGE", key.as_slice(), b"0", b"2"])
        }
        BenchKind::Lpush => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"LPUSH", key.as_slice(), value])
        }
        BenchKind::Rpush => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"RPUSH", key.as_slice(), value])
        }
        BenchKind::Lpop => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"LPOP", key.as_slice()])
        }
        BenchKind::Rpop => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"RPOP", key.as_slice()])
        }
        BenchKind::Llen => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"LLEN", key.as_slice()])
        }
        BenchKind::Lrange => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"LRANGE", key.as_slice(), b"0", b"9"])
        }
        BenchKind::Sadd => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SADD", key.as_slice(), value])
        }
        BenchKind::Srem => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SREM", key.as_slice(), value])
        }
        BenchKind::Scard => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SCARD", key.as_slice()])
        }
        BenchKind::Sismember => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"SISMEMBER", key.as_slice(), value])
        }
        BenchKind::Hset => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"HSET", key.as_slice(), b"field", value])
        }
        BenchKind::Hget => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"HGET", key.as_slice(), b"field"])
        }
        BenchKind::Hgetall => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"HGETALL", key.as_slice()])
        }
        BenchKind::Hincrby => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"HINCRBY", key.as_slice(), b"field", b"1"])
        }
        BenchKind::Zadd => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZADD", key.as_slice(), b"1", value])
        }
        BenchKind::Zrem => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZREM", key.as_slice(), value])
        }
        BenchKind::Zcard => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZCARD", key.as_slice()])
        }
        BenchKind::Zscore => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZSCORE", key.as_slice(), value])
        }
        BenchKind::Zrank => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZRANK", key.as_slice(), value])
        }
        BenchKind::Zrevrank => {
            let key = make_key(key_base, sequence);
            encode_resp_parts(&[b"ZREVRANK", key.as_slice(), value])
        }
    }
}

fn percentile_ms(samples_ns: &[u64], percentile: f64) -> f64 {
    if samples_ns.is_empty() {
        return 0.0;
    }
    let max_index = samples_ns.len() - 1;
    let rank = (max_index as f64 * percentile).round() as usize;
    samples_ns[rank.min(max_index)] as f64 / 1_000_000.0
}

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use bytes::BytesMut;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::args::Args;
use crate::resp::{
    encode_resp_parts, make_key, read_n_fixed_hgetall_responses, read_n_fixed_mget_responses,
    read_n_responses, repeat_payload,
};
use crate::spec::{BenchKind, BenchRun};

const SCRIPT_SET_BODY: &[u8] = b"redis.call('SET', KEYS[1], ARGV[1]); return ARGV[1]";
const SCRIPT_GET_BODY: &[u8] = b"return redis.call('GET', KEYS[1])";
const SETUP_BATCH: usize = 64;

#[derive(Default)]
struct WorkerStats {
    completed: u64,
    lat_samples_ns: Vec<u64>,
}

pub struct BenchResult {
    pub name: String,
    pub scenario: Option<&'static str>,
    pub requests: u64,
    pub clients: usize,
    pub elapsed_secs: f64,
    pub req_per_sec: f64,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub data_size: usize,
    pub pipeline: usize,
    pub random_keys: bool,
    pub keyspace: u64,
}

struct Shared {
    host: String,
    port: u16,
    auth: Option<String>,
    spec: BenchRun,
}

#[derive(Clone, Copy)]
struct ClientPlan {
    client_id: u64,
    quota: u64,
}

pub async fn run_single_benchmark(_args: &Args, spec: BenchRun) -> Result<BenchResult, String> {
    let clients = spec.clients.min(spec.requests as usize).max(1);
    let base = spec.requests / clients as u64;
    let extra = (spec.requests % clients as u64) as usize;

    let shared = Arc::new(Shared {
        host: _args.host.clone(),
        port: _args.port,
        auth: _args.auth.clone(),
        spec,
    });

    let thread_count = _args.threads.min(clients).max(1);
    let mut plans = Vec::with_capacity(clients);
    for client_id in 0..clients {
        let quota = base + u64::from(client_id < extra);
        if quota == 0 {
            continue;
        }
        plans.push(ClientPlan {
            client_id: client_id as u64,
            quota,
        });
    }

    let mut shards = vec![Vec::new(); thread_count];
    for (index, plan) in plans.into_iter().enumerate() {
        shards[index % thread_count].push(plan);
    }

    let start = Instant::now();
    let mut handles = Vec::with_capacity(thread_count);
    for (thread_index, shard) in shards.into_iter().enumerate() {
        if shard.is_empty() {
            continue;
        }
        let cfg = Arc::clone(&shared);
        handles.push(
            thread::Builder::new()
                .name(format!("betterkv-bench-{thread_index}"))
                .spawn(move || run_thread_shard(cfg, shard))
                .map_err(|err| format!("failed to spawn benchmark thread {thread_index}: {err}"))?,
        );
    }

    let mut total_completed = 0u64;
    let mut samples = Vec::<u64>::new();
    for handle in handles {
        let thread_stats = handle
            .join()
            .map_err(|_| "benchmark thread panicked".to_string())??;
        for stats in thread_stats {
            total_completed += stats.completed;
            samples.extend(stats.lat_samples_ns);
        }
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
        name: shared.spec.name.clone(),
        scenario: shared.spec.scenario,
        requests: total_completed,
        clients,
        elapsed_secs,
        req_per_sec: total_completed as f64 / elapsed_secs,
        avg_ms,
        p50_ms,
        p95_ms,
        p99_ms,
        data_size: shared.spec.data_size,
        pipeline: shared.spec.pipeline,
        random_keys: shared.spec.random_keys,
        keyspace: shared.spec.keyspace,
    })
}

fn run_thread_shard(cfg: Arc<Shared>, shard: Vec<ClientPlan>) -> Result<Vec<WorkerStats>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to create worker runtime: {err}"))?;

    runtime.block_on(async move {
        let mut handles = Vec::with_capacity(shard.len());
        for plan in shard {
            let worker_cfg = Arc::clone(&cfg);
            handles.push(tokio::spawn(async move {
                run_worker(plan.client_id, plan.quota, worker_cfg).await
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

    let mut stats = WorkerStats::default();
    let mut sequence = 0u64;

    if !cfg.spec.random_keys {
        let one = build_command(
            cfg.spec.kind,
            key_base.as_bytes(),
            &value,
            0,
            script_sha.as_deref(),
        );
        let full_batch = repeat_payload(&one, cfg.spec.pipeline);
        let mut remaining = quota;

        while remaining > 0 {
            let batch = remaining.min(cfg.spec.pipeline as u64) as usize;
            let started = Instant::now();
            if batch == cfg.spec.pipeline {
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
            read_batch_responses(&cfg.spec, &mut stream, &mut parse_buf, batch).await?;

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
        let batch = remaining.min(cfg.spec.pipeline as u64) as usize;
        let mut payload = Vec::with_capacity(batch * (cfg.spec.data_size + 128));
        for _ in 0..batch {
            let key_slot = random_slot(client_id, sequence, cfg.spec.keyspace);
            let command = build_command(
                cfg.spec.kind,
                key_base.as_bytes(),
                &value,
                key_slot,
                script_sha.as_deref(),
            );
            payload.extend_from_slice(&command);
            sequence = sequence.wrapping_add(1);
        }

        let started = Instant::now();
        stream
            .write_all(&payload)
            .await
            .map_err(|err| format!("write failed: {err}"))?;
        read_batch_responses(&cfg.spec, &mut stream, &mut parse_buf, batch).await?;

        let per_req_ns = (started.elapsed().as_nanos() / batch as u128).min(u128::from(u64::MAX));
        stats.lat_samples_ns.push(per_req_ns as u64);
        stats.completed += batch as u64;
        remaining -= batch as u64;
    }

    Ok(stats)
}

async fn read_batch_responses(
    spec: &BenchRun,
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    batch: usize,
) -> Result<(), String> {
    match spec.kind {
        BenchKind::Mget => read_n_fixed_mget_responses(stream, parse_buf, batch, spec.data_size).await,
        BenchKind::Hgetall => {
            read_n_fixed_hgetall_responses(stream, parse_buf, batch, spec.data_size).await
        }
        _ => read_n_responses(stream, parse_buf, batch).await,
    }
}

async fn setup_worker_state(
    stream: &mut TcpStream,
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
    stream: &mut TcpStream,
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
    let mut pending = 0usize;
    for slot in 0..keyspace {
        let key = make_key(key_base, slot);
        if let Some(setup) = build_setup_command(kind, &key, value) {
            payload.extend_from_slice(&setup);
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
    stream: &mut TcpStream,
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

fn build_setup_command(kind: BenchKind, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
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
        | BenchKind::EvalShaRo => Some(encode_resp_parts(&[b"SET", key, value])),
        BenchKind::Mget => {
            let key2 = related_multi_key(key);
            Some(encode_resp_parts(&[
                b"MSET",
                key,
                value,
                key2.as_slice(),
                value,
            ]))
        }
        BenchKind::Mset => {
            let key2 = related_multi_key(key);
            Some(encode_resp_parts(&[b"DEL", key, key2.as_slice()]))
        }
        BenchKind::Lpop | BenchKind::Rpop | BenchKind::Llen | BenchKind::Lrange => {
            Some(encode_resp_parts(&[b"LPUSH", key, value]))
        }
        BenchKind::Srem | BenchKind::Scard | BenchKind::Sismember => {
            Some(encode_resp_parts(&[b"SADD", key, value]))
        }
        BenchKind::Hget | BenchKind::Hgetall => {
            Some(encode_resp_parts(&[b"HSET", key, b"field", value]))
        }
        BenchKind::Hincrby => Some(encode_resp_parts(&[b"HSET", key, b"field", b"0"])),
        BenchKind::Zrem
        | BenchKind::Zcard
        | BenchKind::Zscore
        | BenchKind::Zrank
        | BenchKind::Zrevrank => Some(encode_resp_parts(&[b"ZADD", key, b"1", value])),
        _ => None,
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

fn related_multi_key(key: &[u8]) -> Vec<u8> {
    let mut related = Vec::with_capacity(key.len() + 3);
    related.extend_from_slice(key);
    related.extend_from_slice(b":m2");
    related
}

fn build_command(
    kind: BenchKind,
    key_base: &[u8],
    value: &[u8],
    key_slot: u64,
    script_sha: Option<&[u8]>,
) -> Vec<u8> {
    match kind {
        BenchKind::PingInline => b"PING\r\n".to_vec(),
        BenchKind::PingMbulk => encode_resp_parts(&[b"PING"]),
        BenchKind::Echo => encode_resp_parts(&[b"ECHO", value]),
        BenchKind::Set => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SET", key.as_slice(), value])
        }
        BenchKind::SetNx => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SETNX", key.as_slice(), value])
        }
        BenchKind::Get => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"GET", key.as_slice()])
        }
        BenchKind::GetSet => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"GETSET", key.as_slice(), value])
        }
        BenchKind::Mset => {
            let key1 = make_key(key_base, key_slot);
            let key2 = related_multi_key(&key1);
            encode_resp_parts(&[b"MSET", key1.as_slice(), value, key2.as_slice(), value])
        }
        BenchKind::Mget => {
            let key1 = make_key(key_base, key_slot);
            let key2 = related_multi_key(&key1);
            encode_resp_parts(&[b"MGET", key1.as_slice(), key2.as_slice()])
        }
        BenchKind::Del => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"DEL", key.as_slice()])
        }
        BenchKind::Exists => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"EXISTS", key.as_slice()])
        }
        BenchKind::Expire => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"EXPIRE", key.as_slice(), b"60"])
        }
        BenchKind::Ttl => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"TTL", key.as_slice()])
        }
        BenchKind::Incr => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"INCR", key.as_slice()])
        }
        BenchKind::IncrBy => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"INCRBY", key.as_slice(), b"3"])
        }
        BenchKind::Decr => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"DECR", key.as_slice()])
        }
        BenchKind::DecrBy => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"DECRBY", key.as_slice(), b"3"])
        }
        BenchKind::Strlen => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"STRLEN", key.as_slice()])
        }
        BenchKind::SetRange => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SETRANGE", key.as_slice(), b"0", value])
        }
        BenchKind::GetRange => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"GETRANGE", key.as_slice(), b"0", b"2"])
        }
        BenchKind::Lpush => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"LPUSH", key.as_slice(), value])
        }
        BenchKind::Rpush => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"RPUSH", key.as_slice(), value])
        }
        BenchKind::Lpop => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"LPOP", key.as_slice()])
        }
        BenchKind::Rpop => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"RPOP", key.as_slice()])
        }
        BenchKind::Llen => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"LLEN", key.as_slice()])
        }
        BenchKind::Lrange => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"LRANGE", key.as_slice(), b"0", b"9"])
        }
        BenchKind::Sadd => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SADD", key.as_slice(), value])
        }
        BenchKind::Srem => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SREM", key.as_slice(), value])
        }
        BenchKind::Scard => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SCARD", key.as_slice()])
        }
        BenchKind::Sismember => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"SISMEMBER", key.as_slice(), value])
        }
        BenchKind::Hset => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"HSET", key.as_slice(), b"field", value])
        }
        BenchKind::Hget => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"HGET", key.as_slice(), b"field"])
        }
        BenchKind::Hgetall => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"HGETALL", key.as_slice()])
        }
        BenchKind::Hincrby => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"HINCRBY", key.as_slice(), b"field", b"1"])
        }
        BenchKind::Zadd => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZADD", key.as_slice(), b"1", value])
        }
        BenchKind::Zrem => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZREM", key.as_slice(), value])
        }
        BenchKind::Zcard => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZCARD", key.as_slice()])
        }
        BenchKind::Zscore => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZSCORE", key.as_slice(), value])
        }
        BenchKind::Zrank => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZRANK", key.as_slice(), value])
        }
        BenchKind::Zrevrank => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"ZREVRANK", key.as_slice(), value])
        }
        BenchKind::Eval => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"EVAL", SCRIPT_SET_BODY, b"1", key.as_slice(), value])
        }
        BenchKind::EvalRo => {
            let key = make_key(key_base, key_slot);
            encode_resp_parts(&[b"EVAL_RO", SCRIPT_GET_BODY, b"1", key.as_slice()])
        }
        BenchKind::EvalSha => {
            let key = make_key(key_base, key_slot);
            let sha = script_sha.expect("missing script sha for EVALSHA benchmark");
            encode_resp_parts(&[b"EVALSHA", sha, b"1", key.as_slice(), value])
        }
        BenchKind::EvalShaRo => {
            let key = make_key(key_base, key_slot);
            let sha = script_sha.expect("missing script sha for EVALSHA_RO benchmark");
            encode_resp_parts(&[b"EVALSHA_RO", sha, b"1", key.as_slice()])
        }
    }
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

fn percentile_ms(samples_ns: &[u64], percentile: f64) -> f64 {
    if samples_ns.is_empty() {
        return 0.0;
    }
    let max_index = samples_ns.len() - 1;
    let rank = (max_index as f64 * percentile).round() as usize;
    samples_ns[rank.min(max_index)] as f64 / 1_000_000.0
}

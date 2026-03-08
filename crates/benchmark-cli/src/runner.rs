use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use bytes::BytesMut;
use tokio::task::JoinSet;

use crate::cli::Config;
use crate::command::{plans, BenchPlan, CommandState};
use crate::connection::RedisConnection;
use crate::report::{self, BenchStats};

pub async fn run(config: Config) -> Result<(), String> {
    if config.cluster {
        return Err("--cluster is not supported yet".to_string());
    }

    let stdin_payload = if config.stdin_last_arg {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).map_err(|err| format!("Failed to read stdin: {err}"))?;
        Some(input)
    } else {
        None
    };

    let plans = plans(&config, stdin_payload);
    if plans.is_empty() {
        return Err("No benchmarks selected".to_string());
    }
    if config.csv {
        report::print_header();
    }

    if config.idle_mode {
        let mut set = JoinSet::new();
        for _ in 0..config.clients {
            let cfg = config.clone();
            set.spawn(async move { RedisConnection::connect(&cfg).await });
        }
        while let Some(result) = set.join_next().await {
            result.map_err(|err| err.to_string())??;
        }
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    loop {
        for plan in &plans {
            run_one(&config, plan).await?;
        }
        if !config.loop_forever {
            break;
        }
    }

    Ok(())
}

async fn run_one(config: &Config, plan: &BenchPlan) -> Result<(), String> {
    let requests = config.requests;
    let counter = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let mut set = JoinSet::new();

    for client_id in 0..config.clients {
        let cfg = config.clone();
        let counter = Arc::clone(&counter);
        let bench = plan.clone();
        set.spawn(async move { run_client(cfg, bench, counter, client_id as u64).await });
    }

    let mut stats = BenchStats::default();
    while let Some(result) = set.join_next().await {
        stats.merge(result.map_err(|err| err.to_string())??);
    }

    report::print(
        &plan.title,
        &stats,
        start.elapsed(),
        requests,
        config.clients,
        config.data_size,
        config.keep_alive,
        config.threads,
        config.quiet,
        config.precision,
        config.csv,
    );

    Ok(())
}

async fn run_client(config: Config, plan: BenchPlan, counter: Arc<AtomicU64>, client_id: u64) -> Result<BenchStats, String> {
    let mut state = CommandState::new(config.seed ^ client_id.rotate_left(7), config.data_size);
    let mut conn = None;
    let mut out = BytesMut::with_capacity(config.pipeline * 256);
    let mut stats = BenchStats::default();

    if config.keep_alive {
        conn = Some(RedisConnection::connect(&config).await?);
    }

    if let Some(setup) = &plan.setup {
        if conn.is_none() {
            conn = Some(RedisConnection::connect(&config).await?);
        }
        out.clear();
        for _ in 0..600 {
            state.encode(setup, config.keyspace_len, &mut out);
        }
        conn.as_mut().expect("setup connection").write_and_drain(&out, 600).await?;
        out.clear();
    }

    loop {
        let start = counter.fetch_add(config.pipeline as u64, Ordering::Relaxed);
        if start >= config.requests {
            break;
        }
        let batch = (config.requests - start).min(config.pipeline as u64) as usize;

        if conn.is_none() {
            conn = Some(RedisConnection::connect(&config).await?);
        }

        out.clear();
        for _ in 0..batch {
            state.encode(&plan.command, config.keyspace_len, &mut out);
        }

        let batch_start = Instant::now();
        let connection = conn.as_mut().expect("active connection");
        connection.write_all(&out).await?;
        match connection.read_responses(batch).await {
            Ok(errors) => stats.errors += errors,
            Err(err) => {
                stats.errors += 1;
                return Err(err);
            }
        }

        let latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0 / batch as f64;
        stats.record(latency_ms, batch);

        if !config.keep_alive {
            conn = None;
        }
    }

    Ok(stats)
}

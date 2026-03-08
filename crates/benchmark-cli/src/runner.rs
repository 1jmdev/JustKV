use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use tokio::task::JoinSet;

use crate::cli::Config;
use crate::command::{encoded_len, plans, BenchPlan, CommandState, PreparedBatch};
use crate::connection::RedisConnection;
use crate::report::{self, BenchStats};

const SETUP_BATCH_SIZE: usize = 600;
const FIRE_AND_FORGET_TARGET_BYTES: usize = 4 * 1024 * 1024;
const FIRE_AND_FORGET_MAX_BATCH: usize = 65_536;

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
    let start = Instant::now();
    let mut set = JoinSet::new();
    let prepared_repeat = prepared_repeat_count(config, plan);
    let prepared_command = Arc::new(PreparedBatch::from_template(&plan.command, prepared_repeat));
    let prepared_setup = Arc::new(
        plan.setup
            .as_ref()
            .and_then(|setup| PreparedBatch::from_template(setup, SETUP_BATCH_SIZE)),
    );
    let base_requests = requests / config.clients as u64;
    let extra_requests = requests % config.clients as u64;

    for client_id in 0..config.clients {
        let cfg = config.clone();
        let bench = plan.clone();
        let prepared_command = Arc::clone(&prepared_command);
        let prepared_setup = Arc::clone(&prepared_setup);
        let assigned_requests = base_requests + u64::from((client_id as u64) < extra_requests);
        set.spawn(async move {
            run_client(
                cfg,
                bench,
                assigned_requests,
                client_id as u64,
                prepared_command,
                prepared_setup,
            )
            .await
        });
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
        config.fire_and_forget,
    );

    Ok(())
}

async fn run_client(
    config: Config,
    plan: BenchPlan,
    requests: u64,
    client_id: u64,
    prepared_command: Arc<Option<PreparedBatch>>,
    prepared_setup: Arc<Option<PreparedBatch>>,
) -> Result<BenchStats, String> {
    if config.fire_and_forget {
        return run_client_fire_and_forget(
            config,
            plan,
            requests,
            client_id,
            prepared_command,
            prepared_setup,
        )
        .await;
    }

    let mut state = CommandState::new(config.seed ^ client_id.rotate_left(7), config.data_size);
    let mut conn = None;
    let mut out = BytesMut::with_capacity(config.pipeline * 256);
    let mut stats = BenchStats::default();
    let mut remaining = requests;

    if config.keep_alive {
        conn = Some(RedisConnection::connect(&config).await?);
    }

    if let Some(setup) = &plan.setup {
        if conn.is_none() {
            conn = Some(RedisConnection::connect(&config).await?);
        }
        let setup_buf = if let Some(prepared) = prepared_setup.as_ref() {
            prepared.slice(SETUP_BATCH_SIZE)
        } else {
            out.clear();
            for _ in 0..SETUP_BATCH_SIZE {
                state.encode(setup, config.keyspace_len, &mut out);
            }
            &out
        };
        conn.as_mut()
            .expect("setup connection")
            .write_and_drain(setup_buf, SETUP_BATCH_SIZE)
            .await?;
        if prepared_setup.is_none() {
            out.clear();
        }
    }

    while remaining != 0 {
        let batch = remaining.min(config.pipeline as u64) as usize;
        remaining -= batch as u64;

        if conn.is_none() {
            conn = Some(RedisConnection::connect(&config).await?);
        }

        let request_buf = if let Some(prepared) = prepared_command.as_ref() {
            prepared.slice(batch)
        } else {
            out.clear();
            for _ in 0..batch {
                state.encode(&plan.command, config.keyspace_len, &mut out);
            }
            &out
        };

        let batch_start = Instant::now();
        let connection = conn.as_mut().expect("active connection");
        connection.write_all(request_buf).await?;
        if !config.fire_and_forget {
            match connection.read_responses(batch).await {
                Ok(errors) => stats.errors += errors,
                Err(err) => {
                    stats.errors += 1;
                    return Err(err);
                }
            }
        }

        let latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0 / batch as f64;
        stats.record(latency_ms, batch);

        if prepared_command.is_none() {
            out.clear();
        }

        if !config.keep_alive {
            conn = None;
        }
    }

    Ok(stats)
}

async fn run_client_fire_and_forget(
    config: Config,
    plan: BenchPlan,
    requests: u64,
    client_id: u64,
    prepared_command: Arc<Option<PreparedBatch>>,
    prepared_setup: Arc<Option<PreparedBatch>>,
) -> Result<BenchStats, String> {
    let mut state = CommandState::new(config.seed ^ client_id.rotate_left(7), config.data_size);
    let mut conn = Some(RedisConnection::connect(&config).await?);
    let mut out = BytesMut::with_capacity(fire_and_forget_buffer_capacity(&config, &plan.command));
    let mut remaining = requests;
    let started = Instant::now();

    if let Some(setup) = &plan.setup {
        let setup_buf = if let Some(prepared) = prepared_setup.as_ref() {
            prepared.slice(SETUP_BATCH_SIZE)
        } else {
            out.clear();
            for _ in 0..SETUP_BATCH_SIZE {
                state.encode(setup, config.keyspace_len, &mut out);
            }
            &out
        };
        conn.as_mut()
            .expect("setup connection")
            .write_and_drain(setup_buf, SETUP_BATCH_SIZE)
            .await?;
        out.clear();
    }

    while remaining != 0 {
        let batch_limit = remaining.min(fire_and_forget_batch_limit(&config, &plan.command) as u64) as usize;
        let request_buf = if let Some(prepared) = prepared_command.as_ref() {
            prepared.slice(batch_limit)
        } else {
            fill_fire_and_forget_batch(
                &mut state,
                &plan.command,
                config.keyspace_len,
                &mut out,
                batch_limit,
            );
            &out
        };

        conn.as_mut()
            .expect("active connection")
            .write_all(request_buf)
            .await?;
        remaining -= batch_limit as u64;
    }

    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let mut stats = BenchStats::default();
    if requests != 0 {
        stats.record(elapsed / requests as f64, requests as usize);
    }
    Ok(stats)
}

fn prepared_repeat_count(config: &Config, plan: &BenchPlan) -> usize {
    if config.fire_and_forget {
        fire_and_forget_batch_limit(config, &plan.command)
    } else {
        config.pipeline
    }
}

fn fire_and_forget_batch_limit(config: &Config, template: &crate::command::CommandTemplate) -> usize {
    let Some(command_len) = encoded_len(template) else {
        return config.pipeline.max(1);
    };

    let target = (FIRE_AND_FORGET_TARGET_BYTES / command_len.max(1)).max(config.pipeline);
    target.min(FIRE_AND_FORGET_MAX_BATCH).max(1)
}

fn fire_and_forget_buffer_capacity(config: &Config, template: &crate::command::CommandTemplate) -> usize {
    encoded_len(template)
        .map(|len| len * fire_and_forget_batch_limit(config, template))
        .unwrap_or(FIRE_AND_FORGET_TARGET_BYTES.max(config.pipeline * 256))
}

fn fill_fire_and_forget_batch(
    state: &mut CommandState,
    template: &crate::command::CommandTemplate,
    keyspace_len: Option<u64>,
    out: &mut BytesMut,
    batch: usize,
) {
    out.clear();
    for _ in 0..batch {
        state.encode(template, keyspace_len, out);
    }
}

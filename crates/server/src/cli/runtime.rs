use betterkv_server::auth::parse_user_directive;
use betterkv_server::config::Config;
use betterkv_server::logging::init_logging;

use crate::cli::action::Cli;
use crate::cli::config::load_config_file_into;

pub(crate) fn run(cli: Cli) -> Result<(), String> {
    let _trace = profiler::scope("server::main::run");
    let mut config = Config::default();

    if let Some(ref path) = cli.config {
        if path == "-" {
            load_stdin_into(&mut config)?;
        } else {
            load_config_file_into(path, &mut config)?;
        }
    }

    if let Some(v) = cli.bind {
        config.bind = v;
    }
    if let Some(v) = cli.port {
        config.port = v;
    }
    if let Some(v) = cli.io_threads {
        config.io_threads = v.max(1);
    }
    if let Some(v) = cli.shards {
        config.shards = v.max(1).next_power_of_two();
    }
    if let Some(hz) = cli.hz {
        if hz > 0 {
            config.sweep_interval_ms = (1000 / hz).max(1);
        }
    }
    if let Some(v) = cli.sweep_interval_ms {
        config.sweep_interval_ms = v;
    }
    if let Some(v) = cli.loglevel {
        config.log_level = v.to_ascii_lowercase();
    }
    if let Some(v) = cli.logfile {
        if v.eq_ignore_ascii_case("stdout") {
            config.log_file = None;
        } else {
            config.log_file = Some(v);
        }
    }
    if let Some(v) = cli.dir {
        config.data_dir = v;
    }
    if let Some(v) = cli.dbfilename {
        config.dbfilename = v;
    }
    if let Some(values) = cli.save {
        config.snapshot_interval_secs = parse_save_interval(&values)?;
    }
    if cli.snapshot_on_shutdown {
        config.snapshot_on_shutdown = true;
    }
    if let Some(v) = cli.requirepass {
        config.requirepass = Some(v);
    }
    if !cli.user.is_empty() {
        config
            .user_directives
            .push(parse_user_directive(&cli.user)?);
    }

    let logging_guard = init_logging(&config)?;
    let _ = &logging_guard.file_guard;
    tracing::info!(
        bind = %config.bind,
        port = config.port,
        io_threads = config.io_threads,
        shards = config.shards,
        requirepass = config.requirepass.is_some(),
        acl_users = config.user_directives.len(),
        snapshot_path = %config.snapshot_path().display(),
        snapshot_interval_secs = config.snapshot_interval_secs,
        "starting betterkv server"
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.io_threads)
        .enable_all()
        .build()
        .map_err(|err| format!("failed to create runtime: {err}"))?;

    let result = runtime
        .block_on(betterkv_server::run(config))
        .map_err(|err| format!("server error: {err}"));
    if let Err(err) = &result {
        tracing::error!(error = %err, "server exited with error");
    } else {
        tracing::info!("server shutdown complete");
    }
    result
}

fn load_stdin_into(config: &mut Config) -> Result<(), String> {
    let _trace = profiler::scope("server::main::load_stdin_into");
    use std::io::Read;
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .map_err(|err| format!("failed to read config from stdin: {err}"))?;
    parse_config_content_into(&content, config)
}

pub(crate) fn parse_config_content_into(content: &str, config: &mut Config) -> Result<(), String> {
    let _trace = profiler::scope("server::main::parse_config_content_into");
    use betterkv_server::auth::parse_user_directive;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let tokens: Vec<String> = trimmed.split_whitespace().map(str::to_string).collect();
        if tokens.is_empty() {
            continue;
        }
        let name = tokens[0].to_ascii_lowercase();
        let values = &tokens[1..];
        match name.as_str() {
            "bind" => config.bind = values[0].clone(),
            "port" => {
                config.port = values[0]
                    .parse::<u16>()
                    .map_err(|_| format!("invalid port '{}'", values[0]))?;
            }
            "io-threads" => {
                config.io_threads = values[0]
                    .parse::<usize>()
                    .map_err(|_| format!("invalid io-threads value '{}'", values[0]))?
                    .max(1);
            }
            "shards" => {
                config.shards = values[0]
                    .parse::<usize>()
                    .map_err(|_| format!("invalid shards value '{}'", values[0]))?
                    .max(1)
                    .next_power_of_two();
            }
            "hz" => {
                let hz = values[0]
                    .parse::<u64>()
                    .map_err(|_| format!("invalid hz value '{}'", values[0]))?;
                if hz > 0 {
                    config.sweep_interval_ms = (1000 / hz).max(1);
                }
            }
            "sweep-interval-ms" => {
                config.sweep_interval_ms = values[0]
                    .parse::<u64>()
                    .map_err(|_| format!("invalid sweep-interval-ms value '{}'", values[0]))?;
            }
            "loglevel" => config.log_level = values[0].to_ascii_lowercase(),
            "logfile" => {
                if values[0].eq_ignore_ascii_case("stdout") {
                    config.log_file = None;
                } else {
                    config.log_file = Some(values[0].clone());
                }
            }
            "dir" => config.data_dir = values[0].clone(),
            "dbfilename" => config.dbfilename = values[0].clone(),
            "save" => {
                // Strip surrounding quotes from each token (e.g. `save ""` > disable).
                let save_values: Vec<String> = tokens[1..]
                    .iter()
                    .map(|t| t.trim_matches('"').to_string())
                    .collect();
                config.snapshot_interval_secs = parse_save_interval(&save_values)?;
            }
            "snapshot-on-shutdown" => {
                config.snapshot_on_shutdown = values[0] == "yes";
            }
            "requirepass" => config.requirepass = Some(values[0].clone()),
            "user" => {
                config
                    .user_directives
                    .push(parse_user_directive(&tokens[1..])?);
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_save_interval(values: &[String]) -> Result<u64, String> {
    let _trace = profiler::scope("server::main::parse_save_interval");
    if values.is_empty() {
        return Ok(0);
    }
    // `--save ""` or `save ""` in config file: empty string disables snapshots.
    if values.len() == 1 && values[0].is_empty() {
        return Ok(0);
    }
    if values.len() == 1 {
        return values[0]
            .parse::<u64>()
            .map_err(|_| format!("invalid save value '{}'", values[0]));
    }
    if values.len() % 2 != 0 {
        return Err("save expects pairs of <seconds> <changes>".to_string());
    }
    let mut interval: Option<u64> = None;
    for chunk in values.chunks(2) {
        let seconds = chunk[0]
            .parse::<u64>()
            .map_err(|_| format!("invalid save seconds '{}'", chunk[0]))?;
        chunk[1]
            .parse::<u64>()
            .map_err(|_| format!("invalid save changes '{}'", chunk[1]))?;
        if seconds == 0 {
            continue;
        }
        interval = Some(match interval {
            Some(current) => current.min(seconds),
            None => seconds,
        });
    }
    Ok(interval.unwrap_or(0))
}

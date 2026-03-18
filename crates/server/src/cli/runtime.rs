use betterkv_server::auth::parse_user_directive;
use betterkv_server::config::{AppendFsync, Config, SaveRule, SnapshotCompression};
use betterkv_server::logging::init_logging;

use crate::cli::action::Cli;
use crate::cli::config::load_config_file_into;

pub(crate) fn run(cli: Cli) -> Result<(), String> {
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
    if let Some(v) = cli.protected_mode {
        config.protected_mode = parse_yes_no(&v, "protected-mode")?;
    }
    if let Some(v) = cli.socket {
        config.socket = Some(v);
    }
    if let Some(v) = cli.io_threads {
        config.io_threads = v.max(1);
    }
    if let Some(v) = cli.shards {
        config.shards = v.max(1).next_power_of_two();
    }
    if let Some(hz) = cli.hz
        && let Some(interval_ms) = 1000_u64.checked_div(hz)
    {
        config.sweep_interval_ms = interval_ms.max(1);
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
        if save_disables_persistence(&values) {
            disable_persistence(&mut config);
        }
        config.save_rules = parse_save_rules(&values)?;
    }
    if cli.snapshot_on_shutdown {
        config.snapshot_on_shutdown = true;
    }
    if let Some(v) = cli.appendonly {
        config.appendonly = parse_yes_no(&v, "appendonly")?;
    }
    if let Some(v) = cli.appendfilename {
        config.appendfilename = v;
    }
    if let Some(v) = cli.appendfsync {
        config.appendfsync = parse_appendfsync(&v)?;
    }
    if let Some(v) = cli.snapshot_compression {
        config.snapshot_compression = parse_snapshot_compression(&v)?;
    }
    if let Some(v) = cli.auto_aof_rewrite_percentage {
        config.auto_aof_rewrite_percentage = v;
    }
    if let Some(v) = cli.auto_aof_rewrite_min_size {
        config.auto_aof_rewrite_min_size = v;
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
        listener = %config.listener_label(),
        io_threads = config.io_threads,
        shards = config.shards,
        protected_mode = config.protected_mode,
        requirepass = config.requirepass.is_some(),
        acl_users = config.user_directives.len(),
        snapshot_path = %config.snapshot_path().display(),
        appendonly = config.appendonly,
        appendonly_path = %config.appendonly_path().display(),
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
    use std::io::Read;
    let mut content = String::new();
    std::io::stdin()
        .read_to_string(&mut content)
        .map_err(|err| format!("failed to read config from stdin: {err}"))?;
    parse_config_content_into(&content, config)
}

pub(crate) fn tokenize_config_line(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let tokens: Vec<String> = trimmed.split_whitespace().map(str::to_string).collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

pub(crate) fn parse_config_content_into(content: &str, config: &mut Config) -> Result<(), String> {
    use betterkv_server::auth::parse_user_directive;

    for line in content.lines() {
        let Some(tokens) = tokenize_config_line(line) else {
            continue;
        };
        let name = tokens[0].to_ascii_lowercase();
        let values = &tokens[1..];
        match name.as_str() {
            "socket" => config.socket = Some(values[0].clone()),
            "bind" => config.bind = values[0].clone(),
            "port" => {
                config.port = values[0]
                    .parse::<u16>()
                    .map_err(|_| format!("invalid port '{}'", values[0]))?;
            }
            "protected-mode" => {
                config.protected_mode = parse_yes_no(values[0].as_str(), "protected-mode")?;
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
                if let Some(interval_ms) = 1000_u64.checked_div(hz) {
                    config.sweep_interval_ms = interval_ms.max(1);
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
                if save_disables_persistence(&save_values) {
                    disable_persistence(config);
                }
                config.save_rules = parse_save_rules(&save_values)?;
            }
            "snapshot-on-shutdown" => {
                config.snapshot_on_shutdown =
                    parse_yes_no(values[0].as_str(), "snapshot-on-shutdown")?;
            }
            "appendonly" => {
                config.appendonly = parse_yes_no(values[0].as_str(), "appendonly")?;
            }
            "appendfilename" => {
                config.appendfilename = values[0].clone();
            }
            "appendfsync" => {
                config.appendfsync = parse_appendfsync(values[0].as_str())?;
            }
            "snapshot-compression" => {
                config.snapshot_compression = parse_snapshot_compression(values[0].as_str())?;
            }
            "auto-aof-rewrite-percentage" => {
                config.auto_aof_rewrite_percentage = values[0]
                    .parse::<u32>()
                    .map_err(|_| format!("invalid auto-aof-rewrite-percentage '{}'", values[0]))?;
            }
            "auto-aof-rewrite-min-size" => {
                config.auto_aof_rewrite_min_size = values[0]
                    .parse::<u64>()
                    .map_err(|_| format!("invalid auto-aof-rewrite-min-size '{}'", values[0]))?;
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

fn parse_save_rules(values: &[String]) -> Result<Vec<SaveRule>, String> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    // `--save ""` or `save ""` in config file: empty string disables snapshots.
    if values.len() == 1 && values[0].is_empty() {
        return Ok(Vec::new());
    }
    if values.len() == 1 {
        let seconds = values[0]
            .parse::<u64>()
            .map_err(|_| format!("invalid save value '{}'", values[0]))?;
        if seconds == 0 {
            return Ok(Vec::new());
        }
        return Ok(vec![SaveRule {
            seconds,
            changes: 1,
        }]);
    }
    if !values.len().is_multiple_of(2) {
        return Err("save expects pairs of <seconds> <changes>".to_string());
    }
    let mut rules = Vec::with_capacity(values.len() / 2);
    for chunk in values.chunks(2) {
        let seconds = chunk[0]
            .parse::<u64>()
            .map_err(|_| format!("invalid save seconds '{}'", chunk[0]))?;
        let changes = chunk[1]
            .parse::<u64>()
            .map_err(|_| format!("invalid save changes '{}'", chunk[1]))?;
        if seconds == 0 {
            continue;
        }
        rules.push(SaveRule { seconds, changes });
    }
    Ok(rules)
}

fn parse_yes_no(value: &str, name: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("invalid {name} value '{value}'")),
    }
}

fn parse_appendfsync(value: &str) -> Result<AppendFsync, String> {
    match value.to_ascii_lowercase().as_str() {
        "always" => Ok(AppendFsync::Always),
        "everysec" => Ok(AppendFsync::EverySec),
        "no" => Ok(AppendFsync::No),
        _ => Err(format!("invalid appendfsync value '{value}'")),
    }
}

fn parse_snapshot_compression(value: &str) -> Result<SnapshotCompression, String> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(SnapshotCompression::None),
        "lz4" => Ok(SnapshotCompression::Lz4),
        _ => Err(format!("invalid snapshot-compression value '{value}'")),
    }
}

fn save_disables_persistence(values: &[String]) -> bool {
    values.len() == 1 && values[0].is_empty()
}

fn disable_persistence(config: &mut Config) {
    config.save_rules.clear();
    config.snapshot_on_shutdown = false;
    config.appendonly = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_mode_defaults_to_enabled() {
        assert!(Config::default().protected_mode);
    }

    #[test]
    fn parse_config_updates_protected_mode() {
        let mut config = Config::default();
        parse_config_content_into("protected-mode no\n", &mut config).expect("parse config");

        assert!(!config.protected_mode);
    }
}

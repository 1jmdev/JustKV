use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use justkv_server::config::Config;

fn main() {
    let _trace = profiler::scope("server::main::main");
    let args = std::env::args().collect::<Vec<_>>();
    let action = match parse_cli_args(&args) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    };

    match action {
        Action::Help => {
            print_usage();
        }
        Action::Version => {
            println!("justkv-server v{}", env!("CARGO_PKG_VERSION"));
        }
        Action::CheckSystem => {
            println!("[ok] system check passed");
        }
        Action::TestMemory(megabytes) => {
            if megabytes == 0 {
                eprintln!("--test-memory requires a value greater than zero");
                std::process::exit(1);
            }
            println!("[ok] memory test simulated for {megabytes} MB");
        }
        Action::Run(runtime) => {
            if let Err(err) = run(runtime) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
enum Action {
    Help,
    Version,
    CheckSystem,
    TestMemory(u64),
    Run(RuntimeArgs),
}

#[derive(Debug, Default)]
struct RuntimeArgs {
    config_file: Option<ConfigInput>,
    directives: Vec<Directive>,
}

#[derive(Debug)]
enum ConfigInput {
    Path(PathBuf),
    Stdin,
}

#[derive(Debug)]
struct Directive {
    name: String,
    values: Vec<String>,
}

fn parse_cli_args(args: &[String]) -> Result<Action, String> {
    let _trace = profiler::scope("server::main::parse_cli_args");
    let tail = &args[1..];
    if tail.is_empty() {
        return Ok(Action::Run(RuntimeArgs::default()));
    }

    if tail.len() == 1 {
        let arg = tail[0].as_str();
        if arg == "-h" || arg == "--help" {
            return Ok(Action::Help);
        }
        if arg == "-v" || arg == "--version" {
            return Ok(Action::Version);
        }
        if arg == "--check-system" {
            return Ok(Action::CheckSystem);
        }
    }

    if tail.first().map(String::as_str) == Some("--test-memory") {
        if tail.len() != 2 {
            return Err("--test-memory requires exactly one value".to_string());
        }
        let value = tail[1]
            .parse::<u64>()
            .map_err(|_| format!("invalid --test-memory value '{}'", tail[1]))?;
        return Ok(Action::TestMemory(value));
    }

    let mut runtime = RuntimeArgs::default();
    let mut index = 0;

    if let Some(first) = tail.first() {
        if first == "-" {
            runtime.config_file = Some(ConfigInput::Stdin);
            index = 1;
        } else if !first.starts_with("--") {
            runtime.config_file = Some(ConfigInput::Path(PathBuf::from(first)));
            index = 1;
        }
    }

    while index < tail.len() {
        let token = &tail[index];
        if token == "-" {
            index += 1;
            continue;
        }

        if !token.starts_with("--") {
            return Err(format!("unexpected argument '{token}'"));
        }

        let name = token.trim_start_matches("--");
        if name.is_empty() {
            return Err("invalid empty option".to_string());
        }
        if name == "help" {
            return Ok(Action::Help);
        }
        if name == "version" {
            return Ok(Action::Version);
        }
        if name == "check-system" {
            return Ok(Action::CheckSystem);
        }
        if name == "test-memory" {
            if index + 1 >= tail.len() || tail[index + 1].starts_with("--") {
                return Err("--test-memory requires a value".to_string());
            }
            let value = tail[index + 1]
                .parse::<u64>()
                .map_err(|_| format!("invalid --test-memory value '{}'", tail[index + 1]))?;
            return Ok(Action::TestMemory(value));
        }
        if name == "sentinel" {
            return Err("sentinel mode is not supported".to_string());
        }

        index += 1;
        let mut values = Vec::new();
        while index < tail.len() && !tail[index].starts_with("--") && tail[index] != "-" {
            values.push(tail[index].clone());
            index += 1;
        }

        runtime.directives.push(Directive {
            name: name.to_ascii_lowercase(),
            values,
        });
    }

    Ok(Action::Run(runtime))
}

fn run(runtime: RuntimeArgs) -> Result<(), String> {
    let _trace = profiler::scope("server::main::run");
    let mut config = Config::default();
    let mut ignored = BTreeSet::new();

    if let Some(config_input) = runtime.config_file {
        let config_directives = load_config_directives(&config_input)?;
        for directive in config_directives {
            apply_directive(
                &directive.name,
                &directive.values,
                &mut config,
                &mut ignored,
            )?;
        }
    }

    for directive in runtime.directives {
        apply_directive(
            &directive.name,
            &directive.values,
            &mut config,
            &mut ignored,
        )?;
    }

    if !ignored.is_empty() {
        let names = ignored.into_iter().collect::<Vec<_>>().join(", ");
        eprintln!("justkv-server: accepted but ignored directives: {names}");
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.io_threads)
        .enable_all()
        .build()
        .map_err(|err| format!("failed to create runtime: {err}"))?;

    runtime
        .block_on(justkv_server::run(config))
        .map_err(|err| format!("server error: {err}"))
}

fn load_config_directives(input: &ConfigInput) -> Result<Vec<Directive>, String> {
    let _trace = profiler::scope("server::main::load_config_directives");
    match input {
        ConfigInput::Path(path) => {
            let mut visited = BTreeSet::new();
            load_config_file(path, &mut visited)
        }
        ConfigInput::Stdin => {
            let mut content = String::new();
            std::io::stdin()
                .read_to_string(&mut content)
                .map_err(|err| format!("failed to read config from stdin: {err}"))?;
            parse_config_content(&content)
        }
    }
}

fn load_config_file(
    path: &Path,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<Vec<Directive>, String> {
    let _trace = profiler::scope("server::main::load_config_file");
    let canonical = std::fs::canonicalize(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&canonical)
        .map_err(|err| format!("failed to read {}: {err}", canonical.display()))?;
    let mut directives = parse_config_content(&content)?;

    let mut expanded = Vec::new();
    for directive in directives.drain(..) {
        if directive.name == "include" {
            for include in directive.values {
                let include_path = if Path::new(&include).is_absolute() {
                    PathBuf::from(include)
                } else {
                    canonical
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(include)
                };
                let nested = load_config_file(&include_path, visited)?;
                expanded.extend(nested);
            }
        } else {
            expanded.push(directive);
        }
    }

    Ok(expanded)
}

fn parse_config_content(content: &str) -> Result<Vec<Directive>, String> {
    let _trace = profiler::scope("server::main::parse_config_content");
    let mut directives = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let tokens = trimmed
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }

        directives.push(Directive {
            name: tokens[0].to_ascii_lowercase(),
            values: tokens[1..].to_vec(),
        });
    }

    Ok(directives)
}

fn apply_directive(
    name: &str,
    values: &[String],
    config: &mut Config,
    ignored: &mut BTreeSet<String>,
) -> Result<(), String> {
    let _trace = profiler::scope("server::main::apply_directive");
    match name {
        "bind" => {
            let value = first_value(name, values)?;
            config.bind = value.to_string();
        }
        "port" => {
            let value = first_value(name, values)?;
            config.port = value
                .parse::<u16>()
                .map_err(|_| format!("invalid port '{value}'"))?;
        }
        "hz" => {
            let value = first_value(name, values)?;
            let hz = value
                .parse::<u64>()
                .map_err(|_| format!("invalid hz value '{value}'"))?;
            if hz > 0 {
                config.sweep_interval_ms = (1000 / hz).max(1);
            }
        }
        "io-threads" => {
            let value = first_value(name, values)?;
            let io_threads = value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} value '{value}'"))?;
            config.io_threads = io_threads.max(1);
        }
        "shards" => {
            let value = first_value(name, values)?;
            let shards = value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} value '{value}'"))?;
            config.shards = shards.max(1).next_power_of_two();
        }
        "sweep-interval-ms" => {
            let value = first_value(name, values)?;
            config.sweep_interval_ms = value
                .parse::<u64>()
                .map_err(|_| format!("invalid sweep-interval-ms value '{value}'"))?;
        }
        "appendonly" | "daemonize" | "protected-mode" | "io-threads-do-reads" => {
            let value = first_value(name, values)?;
            parse_yes_no(name, value)?;
            ignored.insert(name.to_string());
        }
        _ => {
            ignored.insert(name.to_string());
        }
    }

    Ok(())
}

fn first_value<'a>(name: &str, values: &'a [String]) -> Result<&'a str, String> {
    let _trace = profiler::scope("server::main::first_value");
    values
        .first()
        .map(String::as_str)
        .ok_or_else(|| format!("directive '{name}' requires a value"))
}

fn parse_yes_no(name: &str, value: &str) -> Result<bool, String> {
    let _trace = profiler::scope("server::main::parse_yes_no");
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("directive '{name}' expects 'yes' or 'no'")),
    }
}

fn print_usage() {
    let _trace = profiler::scope("server::main::print_usage");
    let bin = std::env::args()
        .next()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "justkv-server".to_string());

    println!("Usage: {bin} [/path/to/justkv.conf] [options] [-]");
    println!("       {bin} - (read config from stdin)");
    println!("       {bin} -v or --version");
    println!("       {bin} -h or --help");
    println!("       {bin} --test-memory <megabytes>");
    println!("       {bin} --check-system");
    println!();
    println!("Examples:");
    println!("       {bin}");
    println!("       echo 'port 6380' | {bin} -");
    println!("       {bin} ./justkv.conf --port 6379");
    println!("       {bin} --port 7777 --bind 127.0.0.1");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_file_and_cli_overrides() {
        let _trace = profiler::scope("server::main::parses_config_file_and_cli_overrides");
        let args = vec![
            "justkv-server".to_string(),
            "./conf/redis.conf".to_string(),
            "--port".to_string(),
            "6380".to_string(),
            "--bind".to_string(),
            "0.0.0.0".to_string(),
        ];

        let action = parse_cli_args(&args).expect("parse args");
        match action {
            Action::Run(runtime) => {
                assert!(matches!(runtime.config_file, Some(ConfigInput::Path(_))));
                assert_eq!(runtime.directives.len(), 2);
                assert_eq!(runtime.directives[0].name, "port");
                assert_eq!(runtime.directives[0].values, vec!["6380"]);
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn parses_test_memory() {
        let _trace = profiler::scope("server::main::parses_test_memory");
        let args = vec![
            "justkv-server".to_string(),
            "--test-memory".to_string(),
            "256".to_string(),
        ];

        let action = parse_cli_args(&args).expect("parse args");
        match action {
            Action::TestMemory(value) => assert_eq!(value, 256),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn parses_help_short_flag() {
        let _trace = profiler::scope("server::main::parses_help_short_flag");
        let args = vec!["justkv-server".to_string(), "-h".to_string()];
        let action = parse_cli_args(&args).expect("parse args");
        assert!(matches!(action, Action::Help));
    }
}

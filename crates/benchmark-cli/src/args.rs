use clap::{ArgAction, Parser};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "justkv-benchmark",
    version,
    disable_help_flag = true,
    disable_version_flag = true,
    about = "Redis-benchmark compatible load tester for JustKV"
)]
pub struct Args {
    #[arg(short = 'h', long = "host", default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short = 'p', long = "port", default_value_t = 6379)]
    pub port: u16,

    #[arg(short = 'a', long = "auth")]
    pub auth: Option<String>,

    #[arg(short = 'c', long = "clients", default_value_t = 50)]
    pub clients: usize,

    #[arg(short = 'n', long = "requests", default_value_t = 100_000)]
    pub requests: u64,

    #[arg(short = 'd', long = "data-size", default_value_t = 3)]
    pub data_size: usize,

    #[arg(short = 'P', long = "pipeline", default_value_t = 1)]
    pub pipeline: usize,

    #[arg(short = 't', long = "tests", value_delimiter = ',', num_args = 1..)]
    pub tests: Vec<String>,

    #[arg(short = 'q', long = "quiet", default_value_t = false)]
    pub quiet: bool,

    #[arg(long = "csv", default_value_t = false)]
    pub csv: bool,

    #[arg(short = 'r', long = "random-keys", default_value_t = false)]
    pub random_keys: bool,

    #[arg(long = "threads", default_value_t = default_threads())]
    pub threads: usize,

    #[arg(long = "help", action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(long = "version", action = ArgAction::Version)]
    pub version: Option<bool>,
}

pub fn validate_args(args: &Args) -> Result<(), String> {
    if args.clients == 0 {
        return Err("--clients must be greater than 0".to_string());
    }
    if args.requests == 0 {
        return Err("--requests must be greater than 0".to_string());
    }
    if args.pipeline == 0 {
        return Err("--pipeline must be greater than 0".to_string());
    }
    if args.threads == 0 {
        return Err("--threads must be greater than 0".to_string());
    }
    Ok(())
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
}

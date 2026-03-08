mod uri;

use std::time::{SystemTime, UNIX_EPOCH};

use clap::{ArgAction, Parser};

pub use uri::apply_uri;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "betterkv-benchmark",
    version,
    trailing_var_arg = true,
    about = "Redis/Valkey benchmark-compatible load tester for BetterKV"
)]
pub struct Args {
    #[arg(short = 'H', long = "host", default_value = "127.0.0.1")]
    pub host: String,
    #[arg(short = 'p', default_value_t = 6379)]
    pub port: u16,
    #[arg(short = 's')]
    pub socket: Option<String>,
    #[arg(short = 'a')]
    pub password: Option<String>,
    #[arg(long = "user", requires = "password")]
    pub user: Option<String>,
    #[arg(short = 'u')]
    pub uri: Option<String>,
    #[arg(short = 'c', default_value_t = 50, value_parser = parse_positive_usize)]
    pub clients: usize,
    #[arg(short = 'n', default_value_t = 100_000, value_parser = parse_positive_u64)]
    pub requests: u64,
    #[arg(short = 'd', default_value_t = 3, value_parser = parse_positive_usize)]
    pub data_size: usize,
    #[arg(long = "dbnum", default_value_t = 0)]
    pub dbnum: u32,
    #[arg(short = '3', action = ArgAction::SetTrue)]
    pub resp3: bool,
    #[arg(long = "threads", value_parser = parse_positive_usize)]
    pub threads: Option<usize>,
    #[arg(long = "cluster", action = ArgAction::SetTrue)]
    pub cluster: bool,
    #[arg(long = "rfr", default_value = "no")]
    pub read_from_replicas: String,
    #[arg(long = "enable-tracking", action = ArgAction::SetTrue)]
    pub enable_tracking: bool,
    #[arg(short = 'k', default_value_t = 1, value_parser = parse_keep_alive)]
    pub keep_alive: u8,
    #[arg(short = 'r', value_parser = parse_positive_u64)]
    pub random_keyspace_len: Option<u64>,
    #[arg(short = 'P', default_value_t = 1, value_parser = parse_positive_usize)]
    pub pipeline: usize,
    #[arg(short = 'q', action = ArgAction::SetTrue)]
    pub quiet: bool,
    #[arg(long = "precision", default_value_t = 3, value_parser = parse_precision)]
    pub precision: usize,
    #[arg(long = "csv", action = ArgAction::SetTrue)]
    pub csv: bool,
    #[arg(short = 'l', action = ArgAction::SetTrue)]
    pub loop_forever: bool,
    #[arg(short = 't', value_delimiter = ',')]
    pub tests: Vec<String>,
    #[arg(short = 'I', action = ArgAction::SetTrue)]
    pub idle_mode: bool,
    #[arg(short = 'x', action = ArgAction::SetTrue)]
    pub read_last_arg_from_stdin: bool,
    #[arg(long = "seed")]
    pub seed: Option<u64>,
    #[arg(long = "num-functions", default_value_t = 10)]
    pub num_functions: usize,
    #[arg(long = "num-keys-in-fcall", default_value_t = 1)]
    pub num_keys_in_fcall: usize,
    #[arg(long = "tls", action = ArgAction::SetTrue)]
    pub tls: bool,
    #[arg(long = "sni")]
    pub sni: Option<String>,
    #[arg(long = "cacert")]
    pub cacert: Option<String>,
    #[arg(long = "cacertdir")]
    pub cacertdir: Option<String>,
    #[arg(long = "insecure", action = ArgAction::SetTrue)]
    pub insecure: bool,
    #[arg(long = "cert")]
    pub cert: Option<String>,
    #[arg(long = "key")]
    pub key: Option<String>,
    #[arg(long = "tls-ciphers")]
    pub tls_ciphers: Option<String>,
    #[arg(long = "tls-ciphersuites")]
    pub tls_ciphersuites: Option<String>,
    #[arg(long = "strict", action = ArgAction::SetTrue, hide = true)]
    pub strict: bool,
    #[arg(long = "no-response-check", action = ArgAction::SetTrue)]
    pub no_response_check: bool,
    #[arg(allow_hyphen_values = true)]
    pub command_args: Vec<String>,
}

impl Args {
    pub fn apply_connection_overrides(&mut self) -> Result<(), String> {
        if let Some(uri) = self.uri.clone() {
            apply_uri(self, &uri)?;
        }
        Ok(())
    }

    pub fn validate_runtime_features(&self) -> Result<(), String> {
        let unsupported = [
            (self.socket.is_some(), "-s / unix sockets"),
            (self.resp3, "-3 / RESP3"),
            (self.cluster, "--cluster"),
            (self.read_from_replicas != "no", "--rfr"),
            (self.enable_tracking, "--enable-tracking"),
            (self.tls, "--tls"),
            (self.sni.is_some(), "--sni"),
            (self.cacert.is_some(), "--cacert"),
            (self.cacertdir.is_some(), "--cacertdir"),
            (self.insecure, "--insecure"),
            (self.cert.is_some(), "--cert"),
            (self.key.is_some(), "--key"),
            (self.tls_ciphers.is_some(), "--tls-ciphers"),
            (self.tls_ciphersuites.is_some(), "--tls-ciphersuites"),
        ];

        if let Some(name) = unsupported
            .into_iter()
            .find_map(|(enabled, name)| enabled.then_some(name))
        {
            return Err(format!("{name} is not supported yet"));
        }

        Ok(())
    }

    pub fn random_seed(&self) -> u64 {
        self.seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_nanos() as u64)
                .unwrap_or(0xBAD5_EED)
        })
    }

    pub fn keep_alive_enabled(&self) -> bool {
        self.keep_alive != 0
    }

    pub fn thread_count(&self) -> usize {
        self.threads.unwrap_or_else(default_threads)
    }
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
}

fn parse_positive_usize(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|err| format!("invalid integer {raw:?}: {err}"))?;
    if value == 0 {
        return Err("value must be greater than 0".to_string());
    }
    Ok(value)
}

fn parse_positive_u64(raw: &str) -> Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|err| format!("invalid integer {raw:?}: {err}"))?;
    if value == 0 {
        return Err("value must be greater than 0".to_string());
    }
    Ok(value)
}

fn parse_keep_alive(raw: &str) -> Result<u8, String> {
    let value = raw
        .parse::<u8>()
        .map_err(|err| format!("invalid integer {raw:?}: {err}"))?;
    if value > 1 {
        return Err("value must be 0 or 1".to_string());
    }
    Ok(value)
}

fn parse_precision(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|err| format!("invalid integer {raw:?}: {err}"))?;
    if value > 9 {
        return Err("value must be between 0 and 9".to_string());
    }
    Ok(value)
}

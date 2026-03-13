use clap::{ArgAction, Parser};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 6379;
const DEFAULT_CLIENTS: usize = 50;
const DEFAULT_REQUESTS: u64 = 100_000;
const DEFAULT_DATA_SIZE: usize = 3;
const DEFAULT_PIPELINE: usize = 1;
const DEFAULT_RANDOM_KEYSPACE: u64 = 10_000;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "betterkv-benchmark",
    version,
    about = "Redis-benchmark style load tester for BetterKV"
)]
pub struct Args {
    #[arg(short = 'H', long = "host", default_value = DEFAULT_HOST)]
    pub host: String,

    #[arg(short = 'p', long = "port", default_value_t = DEFAULT_PORT, value_parser = parse_positive_u16)]
    pub port: u16,

    #[arg(long = "socket")]
    pub socket: Option<String>,

    #[arg(short = 'a')]
    pub password: Option<String>,

    #[arg(long = "user", requires = "password")]
    pub user: Option<String>,

    #[arg(short = 'u')]
    pub uri: Option<String>,

    #[arg(short = 'c', default_value_t = DEFAULT_CLIENTS, value_parser = parse_positive_usize)]
    pub clients: usize,

    #[arg(short = 'n', default_value_t = DEFAULT_REQUESTS, value_parser = parse_positive_u64)]
    pub requests: u64,

    #[arg(short = 'd', default_value_t = DEFAULT_DATA_SIZE, value_parser = parse_positive_usize)]
    pub data_size: usize,

    #[arg(short = 'r', value_parser = parse_positive_u64)]
    pub random_keyspace_len: Option<u64>,

    #[arg(short = 'P', default_value_t = DEFAULT_PIPELINE, value_parser = parse_positive_usize)]
    pub pipeline: usize,

    #[arg(short = 'w', long = "warmup", value_parser = parse_warmup_spec)]
    pub warmup: Option<WarmupSpec>,

    #[arg(short = 't', long = "tests", value_delimiter = ',', num_args = 1..)]
    pub tests: Vec<String>,

    #[arg(long = "list-tests", default_value_t = false)]
    pub list_tests: bool,

    #[arg(short = 'q', action = ArgAction::SetTrue)]
    pub quiet: bool,

    #[arg(long = "csv", action = ArgAction::SetTrue)]
    pub csv: bool,

    #[arg(long = "threads", default_value_t = default_threads(), hide = true, value_parser = parse_positive_usize)]
    pub threads: usize,

    #[arg(long = "strict", default_value_t = false, hide = true)]
    pub strict: bool,

    #[arg(long = "key-prefix", default_value = "betterkv:bench")]
    pub key_prefix: String,
}

pub fn validate_args(args: &Args) -> Result<(), String> {
    args.resolved_connection()?;
    if args.key_prefix.trim().is_empty() {
        return Err("--key-prefix must not be empty".to_string());
    }
    Ok(())
}

impl Args {
    pub fn resolved_connection(&self) -> Result<Connection, String> {
        let mut connection = if let Some(uri) = self.uri.as_deref() {
            parse_redis_uri(uri)?
        } else {
            Connection {
                target: ConnectionTarget::Tcp {
                    host: self.host.clone(),
                    port: self.port,
                },
                user: None,
                password: None,
            }
        };

        if let Some(path) = self.socket.clone() {
            connection.target = ConnectionTarget::Unix { path };
        } else {
            if self.host != DEFAULT_HOST {
                connection.target.set_host(self.host.clone());
            }
            if self.port != DEFAULT_PORT {
                connection.target.set_port(self.port);
            }
        }
        if self.password.is_some() {
            connection.password = self.password.clone();
        }
        if self.user.is_some() {
            connection.user = self.user.clone();
        }

        Ok(connection)
    }

    pub fn random_keys(&self) -> bool {
        self.random_keyspace_len.is_some()
    }

    pub fn keyspace(&self) -> u64 {
        self.random_keyspace_len.unwrap_or(DEFAULT_RANDOM_KEYSPACE)
    }

    pub fn warmup_requests(&self, tracked_requests: u64) -> u64 {
        self.warmup
            .map(|spec| spec.resolve(tracked_requests))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub target: ConnectionTarget,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ConnectionTarget {
    Tcp { host: String, port: u16 },
    Unix { path: String },
}

impl ConnectionTarget {
    fn set_host(&mut self, host: String) {
        if let Self::Tcp { host: current, .. } = self {
            *current = host;
        }
    }

    fn set_port(&mut self, port: u16) {
        if let Self::Tcp { port: current, .. } = self {
            *current = port;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WarmupSpec {
    Absolute(u64),
    Percent(u8),
    Ratio { warmup: u64, tracked: u64 },
}

impl WarmupSpec {
    pub fn resolve(self, tracked_requests: u64) -> u64 {
        match self {
            Self::Absolute(value) => value,
            Self::Percent(value) => tracked_requests.saturating_mul(value as u64) / 100,
            Self::Ratio { warmup, tracked } => {
                tracked_requests.saturating_mul(warmup) / tracked.max(1)
            }
        }
    }
}

fn parse_positive_usize(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|err| format!("invalid positive integer '{raw}': {err}"))?;
    if value == 0 {
        return Err("value must be greater than 0".to_string());
    }
    Ok(value)
}

fn parse_positive_u64(raw: &str) -> Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|err| format!("invalid positive integer '{raw}': {err}"))?;
    if value == 0 {
        return Err("value must be greater than 0".to_string());
    }
    Ok(value)
}

fn parse_positive_u16(raw: &str) -> Result<u16, String> {
    let value = raw
        .parse::<u16>()
        .map_err(|err| format!("invalid port '{raw}': {err}"))?;
    if value == 0 {
        return Err("port must be greater than 0".to_string());
    }
    Ok(value)
}

fn parse_warmup_spec(raw: &str) -> Result<WarmupSpec, String> {
    if let Some(value) = raw.strip_suffix('%') {
        let percent = parse_positive_u64(value)?;
        if percent > 100 {
            return Err("warmup percent must be between 1% and 100%".to_string());
        }
        return Ok(WarmupSpec::Percent(percent as u8));
    }

    if let Some((left, right)) = raw.split_once(':') {
        let warmup = parse_positive_u64(left)?;
        let tracked = parse_positive_u64(right)?;
        return Ok(WarmupSpec::Ratio { warmup, tracked });
    }

    Ok(WarmupSpec::Absolute(parse_positive_u64(raw)?))
}

fn parse_redis_uri(uri: &str) -> Result<Connection, String> {
    let rest = uri
        .strip_prefix("redis://")
        .or_else(|| uri.strip_prefix("rediss://"))
        .ok_or_else(|| "uri must start with redis:// or rediss://".to_string())?;

    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return Err("uri is missing host".to_string());
    }

    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((userinfo, hostport)) => (Some(userinfo), hostport),
        None => (None, authority),
    };

    let (host, port) = if let Some((host, raw_port)) = hostport.rsplit_once(':') {
        if host.is_empty() {
            return Err("uri host must not be empty".to_string());
        }
        (host.to_string(), parse_positive_u16(raw_port)?)
    } else {
        (hostport.to_string(), DEFAULT_PORT)
    };

    let (user, password) = match userinfo {
        Some(info) => {
            let (user, password) = match info.split_once(':') {
                Some((user, password)) => (Some(user.to_string()), Some(password.to_string())),
                None => (None, Some(info.to_string())),
            };
            (
                user.filter(|u| !u.is_empty()),
                password.filter(|p| !p.is_empty()),
            )
        }
        None => (None, None),
    };

    Ok(Connection {
        target: ConnectionTarget::Tcp { host, port },
        user,
        password,
    })
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
}

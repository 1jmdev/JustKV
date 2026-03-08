use std::time::{SystemTime, UNIX_EPOCH};

pub const HELP: &str = r#"Usage: redis-benchmark [OPTIONS] [COMMAND ARGS...]

Options:
 -h <hostname>      Server hostname (default 127.0.0.1)
 -p <port>          Server port (default 6379)
 -s <socket>        Server socket (overrides host and port)
 -a <password>      Password for Redis Auth
 --user <username>  Used to send ACL style 'AUTH username pass'. Needs -a.
 -u <uri>           Server URI on format redis://user:password@host:port/dbnum
                    User, password and dbnum are optional. For authentication
                    without a username, use username 'default'. For TLS, use
                    the scheme 'rediss'.
 -c <clients>       Number of parallel connections (default 50).
 -n <requests>      Total number of requests (default 100000)
 -d <size>          Data size of SET/GET value in bytes (default 3)
 --dbnum <db>       SELECT the specified db number (default 0)
 -3                 Start session in RESP3 protocol mode.
 --threads <num>    Enable multi-thread mode.
 --cluster          Enable cluster mode.
 --enable-tracking  Send CLIENT TRACKING on before starting benchmark.
 -k <boolean>       1=keep alive 0=reconnect (default 1)
 -r <keyspacelen>   Replace __rand_int__ with a 12-digit random integer.
 -P <numreq>        Pipeline <numreq> requests. Default 1 (no pipeline).
 -q                 Quiet. Just show query/sec values
 --precision <num>  Number of decimal places in latency output (default 0)
 --csv              Output in CSV format
 -l                 Loop. Run the tests forever
 -t <tests>         Only run the comma separated list of tests.
 -I                 Idle mode. Just open N idle connections and wait.
 -x                 Read last argument from STDIN.
 --seed <num>       Set the seed for random number generator.
 --help             Output this help and exit.
 --version          Output version and exit.
"#;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub socket: Option<String>,
    pub password: Option<String>,
    pub user: Option<String>,
    pub clients: usize,
    pub requests: u64,
    pub data_size: usize,
    pub dbnum: u32,
    pub resp3: bool,
    pub threads: usize,
    pub cluster: bool,
    pub enable_tracking: bool,
    pub keep_alive: bool,
    pub keyspace_len: Option<u64>,
    pub pipeline: usize,
    pub quiet: bool,
    pub precision: usize,
    pub csv: bool,
    pub loop_forever: bool,
    pub tests: Option<Vec<String>>,
    pub idle_mode: bool,
    pub stdin_last_arg: bool,
    pub seed: u64,
    pub command: Vec<String>,
}

pub enum Action {
    Help,
    Version,
    Run(Config),
}

impl Config {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Action, String> {
        let mut config = Self::default();
        let mut args = args.into_iter().peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" => return Ok(Action::Help),
                "--version" => return Ok(Action::Version),
                "-h" => config.host = next(&mut args, "-h")?,
                "-p" => config.port = parse(next(&mut args, "-p")?, "port")?,
                "-s" => config.socket = Some(next(&mut args, "-s")?),
                "-a" => config.password = Some(next(&mut args, "-a")?),
                "--user" => config.user = Some(next(&mut args, "--user")?),
                "-u" => apply_uri(&mut config, &next(&mut args, "-u")?)?,
                "-c" => config.clients = parse(next(&mut args, "-c")?, "clients")?,
                "-n" => config.requests = parse(next(&mut args, "-n")?, "requests")?,
                "-d" => config.data_size = parse(next(&mut args, "-d")?, "data size")?,
                "--dbnum" => config.dbnum = parse(next(&mut args, "--dbnum")?, "dbnum")?,
                "-3" => config.resp3 = true,
                "--threads" => config.threads = parse(next(&mut args, "--threads")?, "threads")?,
                "--cluster" => config.cluster = true,
                "--enable-tracking" => config.enable_tracking = true,
                "-k" => config.keep_alive = parse_bool(&next(&mut args, "-k")?)?,
                "-r" => config.keyspace_len = Some(parse(next(&mut args, "-r")?, "keyspacelen")?),
                "-P" => config.pipeline = parse(next(&mut args, "-P")?, "pipeline")?,
                "-q" => config.quiet = true,
                "--precision" => {
                    config.precision = parse(next(&mut args, "--precision")?, "precision")?
                }
                "--csv" => config.csv = true,
                "-l" => config.loop_forever = true,
                "-t" => {
                    let value = next(&mut args, "-t")?;
                    config.tests = Some(
                        value
                            .split(',')
                            .map(|item| item.to_ascii_lowercase())
                            .collect(),
                    );
                }
                "-I" => config.idle_mode = true,
                "-x" => config.stdin_last_arg = true,
                "--seed" => config.seed = parse(next(&mut args, "--seed")?, "seed")?,
                _ if arg.starts_with('-') => return Err(format!("Unrecognized option: {arg}")),
                _ => {
                    config.command.push(arg);
                    config.command.extend(args);
                    break;
                }
            }
        }

        if config.password.is_none() && config.user.is_some() {
            return Err("--user requires -a".to_string());
        }
        if config.clients == 0 || config.pipeline == 0 || config.threads == 0 {
            return Err("clients, pipeline, and threads must be greater than 0".to_string());
        }
        if config.stdin_last_arg && config.command.is_empty() {
            return Err("-x requires a command".to_string());
        }

        Ok(Action::Run(config))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            socket: None,
            password: None,
            user: None,
            clients: 50,
            requests: 100_000,
            data_size: 3,
            dbnum: 0,
            resp3: false,
            threads: 1,
            cluster: false,
            enable_tracking: false,
            keep_alive: true,
            keyspace_len: None,
            pipeline: 1,
            quiet: false,
            precision: 0,
            csv: false,
            loop_forever: false,
            tests: None,
            idle_mode: false,
            stdin_last_arg: false,
            seed: default_seed(),
            command: Vec::new(),
        }
    }
}

fn next<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("Missing value for {flag}"))
}

fn parse<T: std::str::FromStr>(raw: String, name: &str) -> Result<T, String> {
    raw.parse::<T>()
        .map_err(|_| format!("Invalid {name}: {raw}"))
}

fn parse_bool(raw: &str) -> Result<bool, String> {
    match raw {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(format!("Invalid boolean: {raw}")),
    }
}

fn apply_uri(config: &mut Config, uri: &str) -> Result<(), String> {
    if let Some(rest) = uri.strip_prefix("redis://") {
        apply_uri_inner(config, rest)
    } else if uri.starts_with("rediss://") {
        Err("rediss:// is not supported yet".to_string())
    } else {
        Err("Only redis:// and rediss:// URIs are accepted".to_string())
    }
}

fn apply_uri_inner(config: &mut Config, raw: &str) -> Result<(), String> {
    let (authority, db_path) = raw.split_once('/').unwrap_or((raw, ""));
    let (userinfo, hostport) = authority
        .rsplit_once('@')
        .map_or((None, authority), |(a, b)| (Some(a), b));

    if let Some(userinfo) = userinfo {
        let (user, pass) = userinfo.split_once(':').unwrap_or((userinfo, ""));
        if !user.is_empty() {
            config.user = Some(user.to_string());
        }
        if !pass.is_empty() {
            config.password = Some(pass.to_string());
        }
    }

    let (host, port) = hostport
        .rsplit_once(':')
        .map_or((hostport, None), |(a, b)| (a, Some(b)));
    if host.is_empty() {
        return Err("Invalid URI host".to_string());
    }
    config.host = host.to_string();
    if let Some(port) = port {
        config.port = port
            .parse()
            .map_err(|_| format!("Invalid URI port: {port}"))?;
    }
    if !db_path.is_empty() {
        config.dbnum = db_path
            .parse()
            .map_err(|_| format!("Invalid URI dbnum: {db_path}"))?;
    }

    Ok(())
}

fn default_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos() as u64)
        .unwrap_or(0x5eed_u64)
}

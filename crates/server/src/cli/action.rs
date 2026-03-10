use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
#[command(
    name = "betterkv-server",
    version,
    about = "BetterKV server",
    long_about = None,
    after_help = "\
Examples:
  betterkv-server                                     (run with default config)
  echo 'port 6380' | betterkv-server -               (read config from stdin)
  betterkv-server /etc/betterkv/6379.conf            (run with config file)
  betterkv-server --port 7777                        (override port)
  betterkv-server /etc/betterkv.conf --loglevel debug
  betterkv-server --port 7777 --bind 127.0.0.1",
    disable_help_flag = true,
    disable_version_flag = true,
)]
pub(crate) struct Cli {
    #[arg(short = 'h', long = "help", action = ArgAction::Help, help = "Print help")]
    pub help: Option<bool>,

    #[arg(short = 'v', long = "version", action = ArgAction::Version, help = "Print version")]
    pub version: Option<bool>,

    #[arg(
        value_name = "CONFIG",
        help = "Path to config file, or - to read from stdin"
    )]
    pub config: Option<String>,

    #[arg(
        long = "bind",
        value_name = "ADDRESS",
        help = "IP address to bind to (default: 127.0.0.1)"
    )]
    pub bind: Option<String>,

    #[arg(
        long = "port",
        value_name = "PORT",
        help = "TCP port to listen on (default: 6379)"
    )]
    pub port: Option<u16>,

    #[arg(
        long = "io-threads",
        value_name = "N",
        help = "Number of I/O worker threads (default: number of CPUs)"
    )]
    pub io_threads: Option<usize>,

    #[arg(
        long = "shards",
        value_name = "N",
        help = "Number of data shards, rounded up to next power of two (default: CPUs * 64)"
    )]
    pub shards: Option<usize>,

    #[arg(
        long = "hz",
        value_name = "HZ",
        help = "Server tick rate; sets sweep interval to 1000/hz ms (default: 4)"
    )]
    pub hz: Option<u64>,

    #[arg(
        long = "sweep-interval-ms",
        value_name = "MS",
        help = "Interval in milliseconds between expiry sweeps (default: 250)"
    )]
    pub sweep_interval_ms: Option<u64>,

    #[arg(
        long = "loglevel",
        value_name = "LEVEL",
        help = "Log verbosity: trace, debug, info, warn, error (default: info)"
    )]
    pub loglevel: Option<String>,

    #[arg(
        long = "logfile",
        value_name = "PATH",
        help = "Log output file path, or stdout to log to stdout (default: stdout)"
    )]
    pub logfile: Option<String>,

    #[arg(
        long = "dir",
        value_name = "PATH",
        help = "Working directory for snapshot files (default: .)"
    )]
    pub dir: Option<String>,

    #[arg(
        long = "dbfilename",
        value_name = "FILENAME",
        help = "Name of the snapshot file (default: dump.bkv)"
    )]
    pub dbfilename: Option<String>,

    #[arg(
        long = "save",
        value_name = "SECONDS [CHANGES]",
        num_args = 1..,
        help = "Snapshot interval in seconds, optionally with a minimum change count"
    )]
    pub save: Option<Vec<String>>,

    #[arg(
        long = "snapshot-on-shutdown",
        action = ArgAction::SetTrue,
        help = "Save a snapshot when the server shuts down"
    )]
    pub snapshot_on_shutdown: bool,

    #[arg(
        long = "appendonly",
        value_name = "yes|no",
        help = "Enable append-only persistence (default: yes)"
    )]
    pub appendonly: Option<String>,

    #[arg(
        long = "appendfilename",
        value_name = "FILENAME",
        help = "Name of the append-only file (default: appendonly.aof)"
    )]
    pub appendfilename: Option<String>,

    #[arg(
        long = "appendfsync",
        value_name = "always|everysec|no",
        help = "AOF fsync policy (default: everysec)"
    )]
    pub appendfsync: Option<String>,

    #[arg(
        long = "snapshot-compression",
        value_name = "lz4|none",
        help = "Snapshot compression codec (default: lz4)"
    )]
    pub snapshot_compression: Option<String>,

    #[arg(
        long = "auto-aof-rewrite-percentage",
        value_name = "PERCENT",
        help = "Rewrite AOF after it grows by this percentage beyond the base size"
    )]
    pub auto_aof_rewrite_percentage: Option<u32>,

    #[arg(
        long = "auto-aof-rewrite-min-size",
        value_name = "BYTES",
        help = "Minimum AOF size before auto rewrite can trigger"
    )]
    pub auto_aof_rewrite_min_size: Option<u64>,

    #[arg(
        long = "requirepass",
        value_name = "PASSWORD",
        help = "Require clients to authenticate with this password"
    )]
    pub requirepass: Option<String>,

    #[arg(
        long = "user",
        value_name = "NAME RULES...",
        num_args = 1..,
        help = "Define an ACL user: --user <name> [on|off] [>password] [~pattern] [+cmd|-cmd]"
    )]
    pub user: Vec<String>,
}

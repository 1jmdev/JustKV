use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "betterkv-prof",
    about = "Profile a Redis command against an embedded betterkv-server instance."
)]
pub struct Args {
    /// Redis command string, e.g. "GET key1" or "SET foo bar".
    pub command: String,

    /// Result mode:
    ///   all   - show every run trace,
    ///   avg   - show the run closest to average,
    ///   best  - show the single fastest run,
    ///   worst - show the single slowest run.
    #[arg(short = 't', long = "type", value_enum, default_value = "all")]
    pub result_type: ResultType,

    /// How many timed runs to execute (after warmup).
    #[arg(short = 'c', long = "count", default_value_t = 1)]
    pub count: usize,

    /// How many warmup runs to execute before profiling starts.
    #[arg(short = 'w', long = "warmup", default_value_t = 0)]
    pub warmup: usize,

    /// Emit plain tab-separated trace output instead of the pretty box layout.
    #[arg(short = 'p', long = "plain", default_value_t = false)]
    pub plain: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ResultType {
    All,
    Avg,
    Best,
    Worst,
}

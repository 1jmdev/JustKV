#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub io_threads: usize,
    pub shards: usize,
    pub sweep_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        let _trace = profiler::scope("server::config::default");
        Self {
            bind: "127.0.0.1".to_string(),
            port: 6379,
            io_threads: default_threads(),
            shards: default_shards(),
            sweep_interval_ms: 250,
        }
    }
}

impl Config {
    pub fn addr(&self) -> String {
        let _trace = profiler::scope("server::config::addr");
        format!("{}:{}", self.bind, self.port)
    }
}

fn default_shards() -> usize {
    let _trace = profiler::scope("server::config::default_shards");
    default_threads().next_power_of_two()
}

fn default_threads() -> usize {
    let _trace = profiler::scope("server::config::default_threads");
    match std::thread::available_parallelism() {
        Ok(value) => value.get().max(1),
        Err(_) => 4,
    }
}

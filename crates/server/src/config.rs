use crate::auth::UserDirectiveConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub io_threads: usize,
    pub shards: usize,
    pub sweep_interval_ms: u64,
    pub log_level: String,
    pub log_file: Option<String>,
    pub data_dir: String,
    pub dbfilename: String,
    pub snapshot_interval_secs: u64,
    pub snapshot_on_shutdown: bool,
    pub requirepass: Option<String>,
    pub user_directives: Vec<UserDirectiveConfig>,
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
            log_level: "info".to_string(),
            log_file: None,
            data_dir: ".".to_string(),
            dbfilename: "dump.bkv".to_string(),
            snapshot_interval_secs: 300,
            snapshot_on_shutdown: true,
            requirepass: None,
            user_directives: Vec::new(),
        }
    }
}

impl Config {
    pub fn addr(&self) -> String {
        let _trace = profiler::scope("server::config::addr");
        format!("{}:{}", self.bind, self.port)
    }

    pub fn snapshot_path(&self) -> std::path::PathBuf {
        let _trace = profiler::scope("server::config::snapshot_path");
        std::path::Path::new(&self.data_dir).join(&self.dbfilename)
    }
}

fn default_shards() -> usize {
    let _trace = profiler::scope("server::config::default_shards");
    println!("{}", default_threads().next_power_of_two());
    default_threads().next_power_of_two()
}

fn default_threads() -> usize {
    let _trace = profiler::scope("server::config::default_threads");
    match std::thread::available_parallelism() {
        Ok(value) => value.get().max(1),
        Err(_) => 4,
    }
}

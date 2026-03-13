use crate::auth::UserDirectiveConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub socket: Option<String>,
    pub bind: String,
    pub port: u16,
    pub io_threads: usize,
    pub shards: usize,
    pub sweep_interval_ms: u64,
    pub log_level: String,
    pub log_file: Option<String>,
    pub data_dir: String,
    pub dbfilename: String,
    pub save_rules: Vec<SaveRule>,
    pub snapshot_on_shutdown: bool,
    pub snapshot_compression: SnapshotCompression,
    pub appendonly: bool,
    pub appendfilename: String,
    pub appendfsync: AppendFsync,
    pub auto_aof_rewrite_percentage: u32,
    pub auto_aof_rewrite_min_size: u64,
    pub requirepass: Option<String>,
    pub user_directives: Vec<UserDirectiveConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveRule {
    pub seconds: u64,
    pub changes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotCompression {
    None,
    Lz4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendFsync {
    Always,
    EverySec,
    No,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket: None,
            bind: "127.0.0.1".to_string(),
            port: 6379,
            io_threads: default_threads(),
            shards: default_shards(),
            sweep_interval_ms: 250,
            log_level: "info".to_string(),
            log_file: None,
            data_dir: ".".to_string(),
            dbfilename: "dump.bkv".to_string(),
            save_rules: vec![
                SaveRule {
                    seconds: 900,
                    changes: 1,
                },
                SaveRule {
                    seconds: 300,
                    changes: 10,
                },
                SaveRule {
                    seconds: 60,
                    changes: 10_000,
                },
            ],
            snapshot_on_shutdown: true,
            snapshot_compression: SnapshotCompression::Lz4,
            appendonly: true,
            appendfilename: "appendonly.aof".to_string(),
            appendfsync: AppendFsync::EverySec,
            auto_aof_rewrite_percentage: 100,
            auto_aof_rewrite_min_size: 64 * 1024 * 1024,
            requirepass: None,
            user_directives: Vec::new(),
        }
    }
}

impl Config {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }

    pub fn listener_label(&self) -> String {
        match self.socket.as_deref() {
            Some(path) => format!("unix:{path}"),
            None => self.addr(),
        }
    }

    pub fn snapshot_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.data_dir).join(&self.dbfilename)
    }

    pub fn appendonly_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.data_dir).join(&self.appendfilename)
    }
}

fn default_shards() -> usize {
    default_threads() * 64
}

fn default_threads() -> usize {
    match std::thread::available_parallelism() {
        Ok(value) => value.get().max(1),
        Err(_) => 4,
    }
}

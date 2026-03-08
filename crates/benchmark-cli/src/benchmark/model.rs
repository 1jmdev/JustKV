use std::sync::atomic::{AtomicBool, AtomicU64};

use crate::resp::ExpectedResponse;
use crate::workload::BenchRun;

#[derive(Default)]
pub(crate) struct WorkerStats {
    pub completed: u64,
    pub latencies_ns: Vec<u64>,
}

pub struct CumulativeBucket {
    pub percent: f64,
    pub latency_ms: f64,
    pub cumulative_count: u64,
}

pub struct BenchResult {
    pub name: String,
    pub requests: u64,
    pub clients: usize,
    pub elapsed_secs: f64,
    pub req_per_sec: f64,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub data_size: usize,
    pub keep_alive: bool,
    pub multi_thread: bool,
    pub(crate) samples_ns: Vec<u64>,
    pub cumulative_distribution: Vec<CumulativeBucket>,
}

pub(crate) struct Shared {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub run: BenchRun,
    pub strict: bool,
}

pub(crate) struct Progress {
    pub completed: AtomicU64,
    pub finished: AtomicBool,
}

#[derive(Clone, Copy)]
pub(crate) struct ClientPlan {
    pub client_id: u64,
    pub quota: u64,
}

#[derive(Clone, Copy)]
pub struct RandomSource {
    state: u64,
}

impl RandomSource {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

pub struct RequestGroup {
    pub payload: Vec<u8>,
    pub expected: Vec<Option<ExpectedResponse>>,
    pub encoded: Vec<Option<Vec<u8>>>,
}

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

#[derive(Clone)]
pub struct ProfilingConfig {
    pub report_interval: Duration,
    pub slow_command_threshold: Duration,
}

impl ProfilingConfig {
    pub fn from_env() -> Option<Self> {
        if !env_flag_enabled("JUSTKV_PROFILE") {
            return None;
        }

        let report_interval_secs = std::env::var("JUSTKV_PROFILE_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);

        let slow_command_ms = std::env::var("JUSTKV_PROFILE_SLOW_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);

        Some(Self {
            report_interval: Duration::from_secs(report_interval_secs),
            slow_command_threshold: Duration::from_millis(slow_command_ms),
        })
    }
}

pub struct LatencyProfiler {
    started_at: Instant,
    config: ProfilingConfig,
    parse_ns: AtomicU64,
    execute_ns: AtomicU64,
    encode_ns: AtomicU64,
    write_ns: AtomicU64,
    command_count: AtomicU64,
    command_stats: Mutex<HashMap<Vec<u8>, CommandStats>>,
}

impl LatencyProfiler {
    pub fn new(config: ProfilingConfig) -> Arc<Self> {
        Arc::new(Self {
            started_at: Instant::now(),
            config,
            parse_ns: AtomicU64::new(0),
            execute_ns: AtomicU64::new(0),
            encode_ns: AtomicU64::new(0),
            write_ns: AtomicU64::new(0),
            command_count: AtomicU64::new(0),
            command_stats: Mutex::new(HashMap::new()),
        })
    }

    pub fn report_interval(&self) -> Duration {
        self.config.report_interval
    }

    pub fn slow_threshold(&self) -> Duration {
        self.config.slow_command_threshold
    }

    pub fn record_parse(&self, elapsed: Duration) {
        self.parse_ns
            .fetch_add(duration_to_nanos(elapsed), Ordering::Relaxed);
    }

    pub fn record_execute(&self, elapsed: Duration) {
        self.execute_ns
            .fetch_add(duration_to_nanos(elapsed), Ordering::Relaxed);
    }

    pub fn record_encode(&self, elapsed: Duration) {
        self.encode_ns
            .fetch_add(duration_to_nanos(elapsed), Ordering::Relaxed);
    }

    pub fn record_write(&self, elapsed: Duration) {
        self.write_ns
            .fetch_add(duration_to_nanos(elapsed), Ordering::Relaxed);
    }

    pub fn record_command(&self, command: &[u8], elapsed: Duration) {
        let elapsed_ns = duration_to_nanos(elapsed);
        self.command_count.fetch_add(1, Ordering::Relaxed);

        let mut key = command.to_vec();
        key.make_ascii_uppercase();

        let mut guard = self.command_stats.lock();
        let stats = guard.entry(key).or_default();
        stats.count += 1;
        stats.total_ns += elapsed_ns;
        stats.max_ns = stats.max_ns.max(elapsed_ns);
        if elapsed >= self.config.slow_command_threshold {
            stats.slow_count += 1;
        }
    }

    pub fn report_and_reset(&self) {
        let parse_ns = self.parse_ns.swap(0, Ordering::Relaxed);
        let execute_ns = self.execute_ns.swap(0, Ordering::Relaxed);
        let encode_ns = self.encode_ns.swap(0, Ordering::Relaxed);
        let write_ns = self.write_ns.swap(0, Ordering::Relaxed);
        let command_count = self.command_count.swap(0, Ordering::Relaxed);

        let mut guard = self.command_stats.lock();
        if command_count == 0 && guard.is_empty() {
            return;
        }

        let mut commands = guard.drain().collect::<Vec<_>>();
        drop(guard);
        commands.sort_by(|a, b| b.1.total_ns.cmp(&a.1.total_ns));

        let total_stage_ns = parse_ns + execute_ns + encode_ns + write_ns;
        let uptime = self.started_at.elapsed().as_secs_f64();

        eprintln!(
            "[latency-profiler] window={}s uptime={:.1}s commands={} parse={:.3}ms execute={:.3}ms encode={:.3}ms write={:.3}ms total_stage={:.3}ms",
            self.config.report_interval.as_secs(),
            uptime,
            command_count,
            nanos_to_millis(parse_ns),
            nanos_to_millis(execute_ns),
            nanos_to_millis(encode_ns),
            nanos_to_millis(write_ns),
            nanos_to_millis(total_stage_ns)
        );

        let top_n = commands.len().min(10);
        for (command, stats) in commands.into_iter().take(top_n) {
            let avg_ns = if stats.count == 0 {
                0
            } else {
                stats.total_ns / stats.count
            };
            let slow_pct = if stats.count == 0 {
                0.0
            } else {
                (stats.slow_count as f64) * 100.0 / (stats.count as f64)
            };
            let command_name = String::from_utf8_lossy(&command);
            eprintln!(
                "[latency-profiler] cmd={} count={} total={:.3}ms avg={:.3}us max={:.3}us slow={} ({:.1}%)",
                command_name,
                stats.count,
                nanos_to_millis(stats.total_ns),
                nanos_to_micros(avg_ns),
                nanos_to_micros(stats.max_ns),
                stats.slow_count,
                slow_pct
            );
        }
    }
}

#[derive(Default)]
struct CommandStats {
    count: u64,
    total_ns: u64,
    max_ns: u64,
    slow_count: u64,
}

fn duration_to_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn nanos_to_millis(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000.0
}

fn nanos_to_micros(nanos: u64) -> f64 {
    nanos as f64 / 1_000.0
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

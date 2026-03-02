use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

#[derive(Clone)]
pub struct ProfilingConfig {
    pub report_interval: Duration,
    pub slow_command_threshold: Duration,
    pub long_request_threshold: Duration,
    pub slow_sample_limit: usize,
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

        let long_request_ms = std::env::var("JUSTKV_PROFILE_LONG_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(slow_command_ms);

        let slow_sample_limit = std::env::var("JUSTKV_PROFILE_SLOW_SAMPLES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(8)
            .min(64);

        Some(Self {
            report_interval: Duration::from_secs(report_interval_secs),
            slow_command_threshold: Duration::from_millis(slow_command_ms),
            long_request_threshold: Duration::from_millis(long_request_ms),
            slow_sample_limit,
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
    request_count: AtomicU64,
    long_request_count: AtomicU64,
    command_stats: Mutex<HashMap<CommandKey, CommandStats>>,
    request_stats: Mutex<HashMap<CommandKey, RequestStats>>,
    slow_requests: Mutex<Vec<SlowRequestSample>>,
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
            request_count: AtomicU64::new(0),
            long_request_count: AtomicU64::new(0),
            command_stats: Mutex::new(HashMap::new()),
            request_stats: Mutex::new(HashMap::new()),
            slow_requests: Mutex::new(Vec::new()),
        })
    }

    pub fn report_interval(&self) -> Duration {
        self.config.report_interval
    }

    pub fn slow_threshold(&self) -> Duration {
        self.config.slow_command_threshold
    }

    pub fn long_request_threshold(&self) -> Duration {
        self.config.long_request_threshold
    }

    pub fn slow_sample_limit(&self) -> usize {
        self.config.slow_sample_limit
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

        let key = CommandKey::from_slice(command);

        let mut guard = self.command_stats.lock();
        let stats = guard.entry(key).or_default();
        stats.count += 1;
        stats.total_ns += elapsed_ns;
        stats.max_ns = stats.max_ns.max(elapsed_ns);
        if elapsed >= self.config.slow_command_threshold {
            stats.slow_count += 1;
        }
    }

    pub fn record_request(
        &self,
        command: &[u8],
        parse_elapsed: Duration,
        execute_elapsed: Duration,
        encode_elapsed: Duration,
    ) {
        let parse_ns = duration_to_nanos(parse_elapsed);
        let execute_ns = duration_to_nanos(execute_elapsed);
        let encode_ns = duration_to_nanos(encode_elapsed);
        let total_ns = parse_ns
            .saturating_add(execute_ns)
            .saturating_add(encode_ns);

        self.request_count.fetch_add(1, Ordering::Relaxed);
        let is_long = total_ns >= duration_to_nanos(self.config.long_request_threshold);
        if is_long {
            self.long_request_count.fetch_add(1, Ordering::Relaxed);
        }

        let command_key = CommandKey::from_slice(command);
        let bottleneck = dominant_stage(parse_ns, execute_ns, encode_ns);

        {
            let mut guard = self.request_stats.lock();
            let stats = guard.entry(command_key.clone()).or_default();
            stats.count += 1;
            stats.parse_ns += parse_ns;
            stats.execute_ns += execute_ns;
            stats.encode_ns += encode_ns;
            stats.total_ns += total_ns;
            stats.max_ns = stats.max_ns.max(total_ns);
            if is_long {
                stats.slow_count += 1;
                stats.record_slow_bottleneck(bottleneck);
            }
        }

        if is_long {
            let mut guard = self.slow_requests.lock();
            guard.push(SlowRequestSample {
                command: command_key,
                parse_ns,
                execute_ns,
                encode_ns,
                total_ns,
                bottleneck,
            });
            guard.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));
            if guard.len() > self.config.slow_sample_limit {
                guard.truncate(self.config.slow_sample_limit);
            }
        }
    }

    pub fn report_and_reset(&self) {
        let parse_ns = self.parse_ns.swap(0, Ordering::Relaxed);
        let execute_ns = self.execute_ns.swap(0, Ordering::Relaxed);
        let encode_ns = self.encode_ns.swap(0, Ordering::Relaxed);
        let write_ns = self.write_ns.swap(0, Ordering::Relaxed);
        let command_count = self.command_count.swap(0, Ordering::Relaxed);
        let request_count = self.request_count.swap(0, Ordering::Relaxed);
        let long_request_count = self.long_request_count.swap(0, Ordering::Relaxed);

        let mut command_guard = self.command_stats.lock();
        let mut request_guard = self.request_stats.lock();
        let mut slow_guard = self.slow_requests.lock();
        if command_count == 0
            && request_count == 0
            && command_guard.is_empty()
            && request_guard.is_empty()
        {
            return;
        }

        let mut commands = command_guard.drain().collect::<Vec<_>>();
        let mut requests = request_guard.drain().collect::<Vec<_>>();
        let mut slow_requests = std::mem::take(&mut *slow_guard);
        drop(command_guard);
        drop(request_guard);
        drop(slow_guard);

        commands.sort_by(|a, b| b.1.total_ns.cmp(&a.1.total_ns));
        requests.sort_by(|a, b| b.1.total_ns.cmp(&a.1.total_ns));
        slow_requests.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));

        let total_stage_ns = parse_ns + execute_ns + encode_ns + write_ns;
        let interval_secs = self.config.report_interval.as_secs_f64();
        let uptime = self.started_at.elapsed().as_secs_f64();

        eprintln!(
            "[latency-profiler] window={}s uptime={:.1}s commands={} requests={} cmd_rps={:.1} req_rps={:.1} parse={:.3}ms execute={:.3}ms encode={:.3}ms write={:.3}ms total_stage={:.3}ms long_requests={}",
            self.config.report_interval.as_secs(),
            uptime,
            command_count,
            request_count,
            command_count as f64 / interval_secs,
            request_count as f64 / interval_secs,
            nanos_to_millis(parse_ns),
            nanos_to_millis(execute_ns),
            nanos_to_millis(encode_ns),
            nanos_to_millis(write_ns),
            nanos_to_millis(total_stage_ns),
            long_request_count,
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
            let command_name = String::from_utf8_lossy(command.as_slice());
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

        let top_request_commands = requests.len().min(10);
        for (command, stats) in requests.into_iter().take(top_request_commands) {
            let avg_ns = if stats.count == 0 {
                0
            } else {
                stats.total_ns / stats.count
            };
            let (hot_stage, hot_count) = stats.hottest_slow_stage();
            let hot_pct = if stats.slow_count == 0 {
                0.0
            } else {
                (hot_count as f64) * 100.0 / (stats.slow_count as f64)
            };
            let command_name = String::from_utf8_lossy(command.as_slice());
            eprintln!(
                "[latency-profiler] req_cmd={} count={} total={:.3}ms avg={:.3}us max={:.3}us slow={} parse={:.3}ms execute={:.3}ms encode={:.3}ms hot={} hot_pct={:.1}",
                command_name,
                stats.count,
                nanos_to_millis(stats.total_ns),
                nanos_to_micros(avg_ns),
                nanos_to_micros(stats.max_ns),
                stats.slow_count,
                nanos_to_millis(stats.parse_ns),
                nanos_to_millis(stats.execute_ns),
                nanos_to_millis(stats.encode_ns),
                hot_stage.as_str(),
                hot_pct,
            );
        }

        for sample in slow_requests {
            let command_name = String::from_utf8_lossy(sample.command.as_slice());
            eprintln!(
                "[latency-profiler] slow_req cmd={} total={:.3}us parse={:.3}us execute={:.3}us encode={:.3}us bottleneck={}",
                command_name,
                nanos_to_micros(sample.total_ns),
                nanos_to_micros(sample.parse_ns),
                nanos_to_micros(sample.execute_ns),
                nanos_to_micros(sample.encode_ns),
                sample.bottleneck.as_str(),
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

#[derive(Default)]
struct RequestStats {
    count: u64,
    total_ns: u64,
    max_ns: u64,
    parse_ns: u64,
    execute_ns: u64,
    encode_ns: u64,
    slow_count: u64,
    slow_parse_hot: u64,
    slow_execute_hot: u64,
    slow_encode_hot: u64,
}

impl RequestStats {
    fn record_slow_bottleneck(&mut self, stage: Stage) {
        match stage {
            Stage::Parse => self.slow_parse_hot += 1,
            Stage::Execute => self.slow_execute_hot += 1,
            Stage::Encode => self.slow_encode_hot += 1,
        }
    }

    fn hottest_slow_stage(&self) -> (Stage, u64) {
        let mut hot = (Stage::Execute, self.slow_execute_hot);
        if self.slow_parse_hot > hot.1 {
            hot = (Stage::Parse, self.slow_parse_hot);
        }
        if self.slow_encode_hot > hot.1 {
            hot = (Stage::Encode, self.slow_encode_hot);
        }
        hot
    }
}

struct SlowRequestSample {
    command: CommandKey,
    parse_ns: u64,
    execute_ns: u64,
    encode_ns: u64,
    total_ns: u64,
    bottleneck: Stage,
}

#[derive(Copy, Clone)]
enum Stage {
    Parse,
    Execute,
    Encode,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Execute => "execute",
            Self::Encode => "encode",
        }
    }
}

fn dominant_stage(parse_ns: u64, execute_ns: u64, encode_ns: u64) -> Stage {
    if parse_ns >= execute_ns && parse_ns >= encode_ns {
        Stage::Parse
    } else if execute_ns >= encode_ns {
        Stage::Execute
    } else {
        Stage::Encode
    }
}

/// Redis command names are always short ASCII.  Using a fixed-size inline
/// buffer avoids a heap allocation on every profiled request.
const MAX_CMD_LEN: usize = 32;

#[derive(Clone, PartialEq, Eq, Hash)]
struct CommandKey {
    len: u8,
    data: [u8; MAX_CMD_LEN],
}

impl CommandKey {
    fn from_slice(command: &[u8]) -> Self {
        let len = command.len().min(MAX_CMD_LEN);
        let mut data = [0u8; MAX_CMD_LEN];
        data[..len].copy_from_slice(&command[..len]);
        data[..len].make_ascii_uppercase();
        Self {
            len: len as u8,
            data,
        }
    }

    fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
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

use std::time::Duration;

const HISTOGRAM_US_BUCKETS: usize = 100_000;

pub struct BenchStats {
    total_count: u64,
    total_ms: f64,
    min_ms: f64,
    max_ms: f64,
    buckets: Box<[u64]>,
    pub errors: usize,
}

impl Default for BenchStats {
    fn default() -> Self {
        Self {
            total_count: 0,
            total_ms: 0.0,
            min_ms: f64::INFINITY,
            max_ms: 0.0,
            buckets: vec![0; HISTOGRAM_US_BUCKETS].into_boxed_slice(),
            errors: 0,
        }
    }
}

impl BenchStats {
    pub fn record(&mut self, latency_ms: f64, count: usize) {
        let count = count as u64;
        self.total_count += count;
        self.total_ms += latency_ms * count as f64;
        self.min_ms = self.min_ms.min(latency_ms);
        self.max_ms = self.max_ms.max(latency_ms);

        let micros = (latency_ms * 1000.0).round() as usize;
        let index = micros.min(self.buckets.len() - 1);
        self.buckets[index] += count;
    }

    pub fn merge(&mut self, other: Self) {
        self.total_count += other.total_count;
        self.total_ms += other.total_ms;
        self.min_ms = self.min_ms.min(other.min_ms);
        self.max_ms = self.max_ms.max(other.max_ms);
        self.errors += other.errors;
        for (dst, src) in self.buckets.iter_mut().zip(other.buckets.iter()) {
            *dst += *src;
        }
    }
}

pub fn print_header() {
    println!("\"test\",\"rps\",\"avg_latency_ms\",\"min_latency_ms\",\"p50_latency_ms\",\"p95_latency_ms\",\"p99_latency_ms\",\"max_latency_ms\"");
}

pub fn print(
    title: &str,
    stats: &BenchStats,
    elapsed: Duration,
    requests: u64,
    clients: usize,
    data_size: usize,
    keep_alive: bool,
    threads: usize,
    quiet: bool,
    precision: usize,
    csv: bool,
) {
    let rps = if elapsed.is_zero() {
        f64::INFINITY
    } else {
        requests as f64 / elapsed.as_secs_f64()
    };
    let avg = if stats.total_count == 0 {
        0.0
    } else {
        stats.total_ms / stats.total_count as f64
    };
    let min = if stats.total_count == 0 {
        0.0
    } else {
        stats.min_ms
    };
    let p50 = percentile(stats, 0.50);
    let p95 = percentile(stats, 0.95);
    let p99 = percentile(stats, 0.99);
    let max = if stats.total_count == 0 {
        0.0
    } else {
        stats.max_ms
    };

    if csv {
        println!("\"{}\",\"{}\",\"{avg:.3}\",\"{min:.3}\",\"{p50:.3}\",\"{p95:.3}\",\"{p99:.3}\",\"{max:.3}\"", title, format_rps(rps));
        return;
    }
    if quiet {
        println!(
            "{}: {} requests per second, p50={} msec",
            title,
            format_rps(rps),
            format_latency(p50, precision.max(3))
        );
        return;
    }

    println!("====== {title} ======");
    println!(
        "  {requests} requests completed in {:.2} seconds",
        elapsed.as_secs_f64()
    );
    println!("  {clients} parallel clients");
    println!("  {data_size} bytes payload");
    println!("  keep alive: {}", if keep_alive { 1 } else { 0 });
    println!("  multi-thread: {}", if threads > 1 { "yes" } else { "no" });
    if stats.errors != 0 {
        println!("  errors: {}", stats.errors);
    }
    println!();
    println!("Summary:");
    println!(
        "  throughput summary: {} requests per second",
        format_rps(rps)
    );
    println!("  latency summary (msec):");
    println!("          avg       min       p50       p95       p99       max");
    println!(
        "  {:>10} {:>9} {:>9} {:>9} {:>9} {:>9}",
        format_summary_latency(avg),
        format_summary_latency(min),
        format_summary_latency(p50),
        format_summary_latency(p95),
        format_summary_latency(p99),
        format_summary_latency(max),
    );
    println!();
}

fn percentile(stats: &BenchStats, p: f64) -> f64 {
    if stats.total_count == 0 {
        return 0.0;
    }

    let target = ((stats.total_count - 1) as f64 * p).round() as u64;
    let mut seen = 0_u64;
    for (micros, count) in stats.buckets.iter().enumerate() {
        seen += *count;
        if seen > target {
            return micros as f64 / 1000.0;
        }
    }

    stats.max_ms
}

fn format_rps(value: f64) -> String {
    if value.is_infinite() {
        "inf".to_string()
    } else {
        format!("{value:.2}")
    }
}

fn format_latency(value: f64, precision: usize) -> String {
    format!("{value:.precision$}")
}

fn format_summary_latency(value: f64) -> String {
    format!("{value:.3}")
}

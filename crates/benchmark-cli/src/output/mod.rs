use crate::benchmark::{BenchResult, CumulativeBucket};
use crate::cli::Args;

const PERCENTILES: &[f64] = &[
    0.0, 50.0, 75.0, 87.5, 93.75, 96.875, 98.438, 99.219, 99.609, 99.805, 99.902, 99.951, 99.976,
    99.988, 99.994, 99.998, 100.0,
];

pub fn render_result(args: &Args, result: &BenchResult) {
    if args.csv {
        println!("\"{}\",{:.2}", result.name, result.req_per_sec);
        return;
    }

    if args.quiet {
        println!(
            "{}: {:.2} requests per second",
            result.name, result.req_per_sec
        );
        return;
    }

    println!("====== {} ======", result.name);
    println!(
        "  {} requests completed in {:.2} seconds",
        result.requests, result.elapsed_secs
    );
    println!("  {} parallel clients", result.clients);
    println!("  {} bytes payload", result.data_size);
    println!("  keep alive: {}", if result.keep_alive { 1 } else { 0 });
    println!(
        "  multi-thread: {}",
        if result.multi_thread { "yes" } else { "no" }
    );
    println!();

    println!("Latency by percentile distribution:");
    for percentile in PERCENTILES {
        let latency_ms = result.latency_for_percentile(*percentile);
        let cumulative = result.cumulative_count_for_percentile(*percentile);
        println!(
            "{percentile:>7.3}% <= {latency:>7} milliseconds (cumulative count {cumulative})",
            latency = format_float(latency_ms, args.precision),
        );
    }
    println!();

    println!("Cumulative distribution of latencies:");
    for bucket in &result.cumulative_distribution {
        println!(
            "{percent:>7.3}% <= {latency:>7} milliseconds (cumulative count {count})",
            percent = bucket.percent,
            latency = format_float(bucket.latency_ms, args.precision),
            count = bucket.cumulative_count,
        );
    }
    println!();

    println!("Summary:");
    println!(
        "  throughput summary: {:.2} requests per second",
        result.req_per_sec
    );
    println!("  latency summary (msec):");
    println!("          avg       min       p50       p95       p99       max");
    println!(
        "  {:>10} {:>9} {:>9} {:>9} {:>9} {:>9}",
        format_float(result.avg_ms, args.precision),
        format_float(result.min_ms, args.precision),
        format_float(result.p50_ms, args.precision),
        format_float(result.p95_ms, args.precision),
        format_float(result.p99_ms, args.precision),
        format_float(result.max_ms, args.precision),
    );
    println!();
}

pub fn progress_line(name: &str, completed: u64, total: u64, elapsed_secs: f64) -> String {
    let percent = if total == 0 {
        0.0
    } else {
        completed as f64 * 100.0 / total as f64
    };
    let rps = if elapsed_secs > 0.0 {
        completed as f64 / elapsed_secs
    } else {
        0.0
    };

    format!("{name}: {completed}/{total} ({percent:.1}%) {rps:.2} requests per second")
}

fn format_float(value: f64, precision: usize) -> String {
    format!("{value:.precision$}")
}

#[allow(dead_code)]
fn _keep_type(_: &CumulativeBucket) {}

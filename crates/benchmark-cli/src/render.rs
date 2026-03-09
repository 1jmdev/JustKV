use crate::args::Args;
use crate::bench::{BenchResult, LatencyObservation};
use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};

pub fn render_result(args: &Args, result: &BenchResult) {
    if args.csv {
        println!(
            "{},{},{},{},{:.6},{:.2},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{},{},{}",
            result.name,
            result.requests,
            result.warmup_requests,
            result.clients,
            result.elapsed_secs,
            result.req_per_sec,
            result.avg_ms,
            result.min_ms,
            result.p50_ms,
            result.p95_ms,
            result.p99_ms,
            result.max_ms,
            result.pipeline,
            result.data_size,
            result.random_keys,
            result.keyspace,
        );
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
    print_run_overview(args, result);
    print_latency_distributions(result);
    print_summary_table(result);
    println!();
}

fn print_run_overview(args: &Args, result: &BenchResult) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Requests",
        "Time",
        "Clients",
        "Pipeline",
        "Payload",
        "Validation",
    ]);
    table.add_row(vec![
        Cell::new(result.requests),
        Cell::new(format!("{:.2}s", result.elapsed_secs)),
        Cell::new(result.clients),
        Cell::new(result.pipeline),
        Cell::new(format!("{} B", result.data_size)),
        Cell::new(if args.strict { "strict" } else { "throughput" }),
    ]);
    println!("{table}");

    if result.warmup_requests > 0 {
        println!("Warmup: {} requests", result.warmup_requests);
    }
    println!(
        "Key pattern: {} (keyspace {}) | Threads: {} | Keep alive: 1",
        if result.random_keys {
            "random"
        } else {
            "fixed"
        },
        result.keyspace,
        if args.threads > 1 {
            args.threads.to_string()
        } else {
            "1".to_string()
        }
    );
}

fn print_summary_table(result: &BenchResult) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Throughput", "Avg", "Min", "P50", "P95", "P99", "Max"]);
    table.add_row(vec![
        Cell::new(format_rps_full(result.req_per_sec)),
        Cell::new(format_latency(result.avg_ms)),
        Cell::new(format_latency(result.min_ms)),
        Cell::new(format_latency(result.p50_ms)),
        Cell::new(format_latency(result.p95_ms)),
        Cell::new(format_latency(result.p99_ms)),
        Cell::new(format_latency(result.max_ms)),
    ]);
    println!("\nSummary:");
    println!("{table}");
}

fn print_latency_distributions(result: &BenchResult) {
    if result.latencies.is_empty() {
        return;
    }

    const PCTS: &[f64] = &[
        0.0, 50.0, 75.0, 87.5, 93.75, 96.875, 98.438, 99.219, 99.609, 99.805, 99.902, 99.951,
        99.976, 99.988, 99.994, 99.997, 99.999, 100.0,
    ];
    const CDF_BINS_MS: &[f64] = &[0.1, 0.2, 0.3, 0.4, 0.5];

    println!("\nLatency by percentile distribution:");
    for pct in PCTS {
        let (ms, count) = percentile_value_and_count(&result.latencies, *pct);
        println!("{pct:.3}% <= {ms:.3} milliseconds (cumulative count {count})");
    }

    println!("\nCumulative distribution of latencies:");
    for max_ms in CDF_BINS_MS {
        let count = cumulative_count_at_or_below(&result.latencies, *max_ms);
        let pct = count as f64 * 100.0 / result.requests as f64;
        println!("{pct:.3}% <= {max_ms:.3} milliseconds (cumulative count {count})");
    }
}

fn percentile_value_and_count(latencies: &[LatencyObservation], pct: f64) -> (f64, u64) {
    let total = latencies
        .iter()
        .fold(0u64, |sum, entry| sum.saturating_add(entry.request_count));
    if total == 0 {
        return (0.0, 0);
    }

    let rank = if pct <= 0.0 {
        0u64
    } else if pct >= 100.0 {
        total - 1
    } else {
        ((pct / 100.0) * (total as f64 - 1.0)).round() as u64
    };

    let mut seen = 0u64;
    for entry in latencies {
        seen = seen.saturating_add(entry.request_count);
        if seen > rank {
            return (entry.per_request_ns as f64 / 1_000_000.0, seen);
        }
    }

    let last = latencies.last().copied().unwrap_or(LatencyObservation {
        per_request_ns: 0,
        request_count: 0,
    });
    (last.per_request_ns as f64 / 1_000_000.0, total)
}

fn cumulative_count_at_or_below(latencies: &[LatencyObservation], max_ms: f64) -> u64 {
    let cutoff_ns = max_ms * 1_000_000.0;
    latencies.iter().fold(0u64, |sum, entry| {
        if entry.per_request_ns as f64 <= cutoff_ns {
            sum.saturating_add(entry.request_count)
        } else {
            sum
        }
    })
}

fn format_latency(ms: f64) -> String {
    if ms >= 1.0 {
        format!("{ms:.3} ms")
    } else if ms >= 0.001 {
        format!("{:.3} us", ms * 1_000.0)
    } else {
        format!("{:.3} ns", ms * 1_000_000.0)
    }
}

fn format_rps_full(rps: f64) -> String {
    format!("{rps:.2} req/s")
}

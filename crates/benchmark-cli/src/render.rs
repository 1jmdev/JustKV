use crate::args::Args;
use crate::bench::BenchResult;

pub fn render_result(args: &Args, result: &BenchResult) {
    if args.csv {
        println!(
            "{},{},{},{:.6},{:.2},{:.4},{:.4},{:.4},{:.4}",
            result.name,
            result.requests,
            result.clients,
            result.elapsed_secs,
            result.req_per_sec,
            result.avg_ms,
            result.p50_ms,
            result.p95_ms,
            result.p99_ms,
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
    println!(
        "  {} requests completed in {:.2} seconds",
        result.requests, result.elapsed_secs
    );
    println!("  {} parallel clients", result.clients);
    println!("  {} bytes payload", args.data_size);
    println!("  {} pipeline depth", args.pipeline);
    println!(
        "  latency avg/p50/p95/p99 = {:.4}/{:.4}/{:.4}/{:.4} ms",
        result.avg_ms, result.p50_ms, result.p95_ms, result.p99_ms
    );
    println!("  {:.2} requests per second", result.req_per_sec);
    println!();
}

mod args;
mod bench;
mod render;
mod resp;
mod spec;

use std::process::ExitCode;

use clap::Parser;

use args::{Args, validate_args};
use bench::run_single_benchmark;
use render::render_result;
use spec::parse_specs;

#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    let args = Args::parse();
    if let Err(err) = validate_args(&args) {
        eprintln!("justkv-benchmark: {err}");
        return ExitCode::FAILURE;
    }

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("justkv-benchmark: failed to create runtime: {err}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("justkv-benchmark: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let specs = parse_specs(&args.tests)?;

    if args.csv {
        println!("test,requests,clients,seconds,rps,avg_ms,p50_ms,p95_ms,p99_ms");
    }

    for spec in specs {
        let result = run_single_benchmark(&args, spec).await?;
        render_result(&args, &result);
    }

    Ok(())
}

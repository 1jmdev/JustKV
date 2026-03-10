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
use spec::{resolve_workload, tests};

#[global_allocator]
static GLOBAL_ALLOCATOR: betterkv_alloc::BetterKvAllocator = betterkv_alloc::BetterKvAllocator;

fn main() -> ExitCode {
    let args = Args::parse();
    if let Err(err) = validate_args(&args) {
        eprintln!("betterkv-benchmark: {err}");
        return ExitCode::FAILURE;
    }

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("betterkv-benchmark: failed to create runtime: {err}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("betterkv-benchmark: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let connection = args.resolved_connection()?;

    if args.list_tests {
        for spec in tests() {
            println!("{}", spec.name);
        }
        return Ok(());
    }

    let workload = resolve_workload(&args)?;

    if args.csv {
        println!(
            "test,requests,warmup,clients,seconds,rps,avg_ms,min_ms,p50_ms,p95_ms,p99_ms,max_ms,pipeline,data_size,random_keys,keyspace"
        );
    }

    for spec in workload {
        let result = run_single_benchmark(&args, &connection, spec).await?;
        render_result(&args, &result);
    }

    Ok(())
}

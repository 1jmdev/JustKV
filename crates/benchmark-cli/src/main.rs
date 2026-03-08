use std::io::Read;
use std::process::ExitCode;

use clap::Parser;

use betterkv_benchmark::benchmark::{
    maybe_warn_about_server_config, run_idle_mode, run_single_benchmark,
};
use betterkv_benchmark::cli::Args;
use betterkv_benchmark::output::render_result;
use betterkv_benchmark::workload::resolve_workload;

#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    let mut args = Args::parse();

    if let Err(err) = args.apply_connection_overrides() {
        eprintln!("betterkv-benchmark: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = args.validate_runtime_features() {
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
    let stdin_last_arg = read_stdin_if_requested(&args)?;

    if args.idle_mode {
        return run_idle_mode(&args).await;
    }

    maybe_warn_about_server_config(&args).await;

    let workload = resolve_workload(&args, stdin_last_arg)?;
    loop {
        for run in workload.clone() {
            let result = run_single_benchmark(&args, run).await?;
            render_result(&args, &result);
        }

        if !args.loop_forever {
            break;
        }
    }

    Ok(())
}

fn read_stdin_if_requested(args: &Args) -> Result<Option<Vec<u8>>, String> {
    if !args.read_last_arg_from_stdin {
        return Ok(None);
    }

    let mut stdin = Vec::new();
    std::io::stdin()
        .read_to_end(&mut stdin)
        .map_err(|err| format!("failed to read stdin: {err}"))?;

    while stdin
        .last()
        .copied()
        .is_some_and(|byte| matches!(byte, b'\n' | b'\r'))
    {
        stdin.pop();
    }

    Ok(Some(stdin))
}

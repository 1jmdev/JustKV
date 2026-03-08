mod cli;
mod command;
mod connection;
mod report;
mod runner;

use std::process::ExitCode;

use cli::{Action, Config};

#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    match Config::parse(std::env::args().skip(1)) {
        Ok(Action::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Action::Version) => {
            println!("betterkv-benchmark {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Action::Run(config)) => {
            let runtime = if config.threads <= 1 {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
            } else {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(config.threads)
                    .thread_name("betterkv-bench")
                    .build()
            };
            let runtime = match runtime {
                Ok(runtime) => runtime,
                Err(err) => {
                    eprintln!("Failed to create runtime: {err}");
                    return ExitCode::FAILURE;
                }
            };

            match runtime.block_on(runner::run(config)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

mod app;
mod cli;
mod client;
mod command;
mod output;
mod repl;

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let _trace = profiler::scope("cli::main::main");
    match app::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

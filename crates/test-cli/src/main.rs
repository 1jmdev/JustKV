mod args;
mod client;
mod discovery;
mod model;
mod output;
mod parser;
mod runner;
mod syntax;

use std::process::ExitCode;

use clap::Parser;

use crate::args::Args;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();

    match runner::run(args).await {
        Ok(summary) => {
            output::print_warnings(&summary.warnings);
            output::print_summary(&summary);
            if summary.failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("betterkv-tester: {err}");
            ExitCode::FAILURE
        }
    }
}

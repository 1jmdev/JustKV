mod cli;

use clap::Parser;
use cli::{Cli, run};

fn main() {
    let _trace = profiler::scope("server::main::main");
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

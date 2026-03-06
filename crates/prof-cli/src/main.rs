#[cfg(not(feature = "profiling"))]
fn main() {
    eprintln!(
        "betterkv-prof: profiling support is not compiled in.\n\
         Rebuild with: cargo build --release -p betterkv-prof --features profiling"
    );
    std::process::exit(1);
}

#[cfg(feature = "profiling")]
mod cli;
#[cfg(feature = "profiling")]
mod command;
#[cfg(feature = "profiling")]
mod net;
#[cfg(feature = "profiling")]
mod render;
#[cfg(feature = "profiling")]
mod session;

#[cfg(feature = "profiling")]
use std::process::ExitCode;

#[cfg(feature = "profiling")]
use clap::Parser;

#[cfg(feature = "profiling")]
fn main() -> ExitCode {
    let args = cli::Args::parse();

    if args.count == 0 {
        eprintln!("betterkv-prof: --count must be > 0");
        return ExitCode::FAILURE;
    }

    let argv = match command::parse_command(&args.command) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("betterkv-prof: {err}");
            return ExitCode::FAILURE;
        }
    };

    let port = match net::find_free_port() {
        Some(p) => p,
        None => {
            eprintln!("betterkv-prof: no free TCP port available");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("betterkv-prof: failed to create runtime: {err}");
            return ExitCode::FAILURE;
        }
    };

    let ok = runtime.block_on(session::run_profile_session(args, argv, port));
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

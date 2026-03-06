use std::time::Duration;

use betterkv_server::config::Config;
use betterkv_server::profile::{ProfileHub, ReportKind};
use betterkv_server::{render_result_plain, render_result_pretty};
use tokio::net::TcpStream;

use crate::cli::{Args, ResultType};
use crate::net::{send_recv, wait_for_server};
use crate::render::{eprint_section, render_responses, render_summary};

#[derive(Clone, Debug)]
pub struct RunResult {
    pub rtt_ns: u64,
    pub response: String,
    pub index: usize,
}

pub async fn run_profile_session(args: Args, argv: Vec<Vec<u8>>, port: u16) -> bool {
    let profile_hub = ProfileHub::capturing();

    let mut config = Config::default();
    config.bind = "127.0.0.1".to_string();
    config.port = port;
    config.io_threads = 1;

    let server_handle = tokio::spawn(betterkv_server::run_with_profile(
        config,
        profile_hub.clone(),
    ));

    if let Err(err) = wait_for_server(port, Duration::from_secs(5)).await {
        eprintln!("betterkv-prof: {err}");
        if server_handle.is_finished() {
            match server_handle.await {
                Ok(Err(e)) => eprintln!("betterkv-prof: embedded server failed: {e}"),
                Ok(Ok(())) | Err(_) => {}
            }
        } else {
            server_handle.abort();
        }
        return false;
    }

    // Persistent connection — reconnecting per-run adds ~10-20µs of loopback
    // TCP setup that is unrelated to server processing time.
    let mut conn = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(s) => s,
        Err(err) => {
            eprintln!("betterkv-prof: connect: {err}");
            server_handle.abort();
            return false;
        }
    };

    if args.warmup > 0 {
        profile_hub.set_enabled(false);
        for i in 0..args.warmup {
            if let Err(err) = send_recv(&mut conn, &argv).await {
                eprintln!("betterkv-prof: warmup run #{} failed: {err}", i + 1);
                server_handle.abort();
                return false;
            }
        }
    }

    profile_hub.reset();
    profile_hub.set_enabled(true);

    eprint_section("Profiling", args.count);

    let mut results = Vec::with_capacity(args.count);
    for index in 1..=args.count {
        match send_recv(&mut conn, &argv).await {
            Ok((rtt_ns, response)) => results.push(RunResult {
                rtt_ns,
                response,
                index,
            }),
            Err(err) => {
                eprintln!("betterkv-prof: run #{index} failed: {err}");
                server_handle.abort();
                return false;
            }
        }
    }

    match profile_hub.selected_runs(to_report_kind(&args.result_type)) {
        Ok(runs) => {
            for run in &runs {
                if args.plain {
                    render_result_plain(run);
                } else {
                    render_result_pretty(run);
                }
            }
        }
        Err(err) => {
            eprintln!("betterkv-prof: {err}");
            server_handle.abort();
            return false;
        }
    }

    render_summary(&results);
    render_responses(&results, &args.result_type);

    server_handle.abort();
    true
}

fn to_report_kind(result_type: &ResultType) -> ReportKind {
    match result_type {
        ResultType::All => ReportKind::All,
        ResultType::Avg => ReportKind::Avg,
        ResultType::Best => ReportKind::Best,
        ResultType::Worst => ReportKind::Worst,
    }
}

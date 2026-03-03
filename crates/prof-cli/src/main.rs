use std::process::{ExitCode, Stdio};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use clap::{Parser, ValueEnum};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use protocol::encoder;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};

/// Profile a Redis command against a private justkv-server instance and display
/// the profiler call-tree for every timed run.
///
/// Examples:
///   prof-cli "GET key1"
///   prof-cli -c 10 -w 3 "SET foo bar"
///   prof-cli -t best -c 5 "HGET myhash field"
///   prof-cli --plain "GET key1"
#[derive(Parser, Debug)]
#[command(name = "prof-cli")]
struct Args {
    /// Redis command string, e.g. "GET key1" or "SET foo bar".
    command: String,

    /// Result mode:
    ///   all   – show every run's trace after warmup,
    ///   avg   – show the run closest to the average,
    ///   best  – show the single fastest run,
    ///   worst – show the single slowest run.
    #[arg(short = 't', long = "type", value_enum, default_value = "all")]
    result_type: ResultType,

    /// How many timed runs to execute (after warmup).
    #[arg(short = 'c', long = "count", default_value_t = 1)]
    count: usize,

    /// How many warmup runs to perform before timing starts (traces suppressed).
    #[arg(short = 'w', long = "warmup", default_value_t = 0)]
    warmup: usize,

    /// Emit plain tab-separated trace output instead of the pretty box layout.
    #[arg(short = 'p', long = "plain", default_value_t = false)]
    plain: bool,

    /// Path to the justkv-server binary.
    #[arg(long = "server", default_value = "./target/release/justkv-server")]
    server_bin: String,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum ResultType {
    All,
    Avg,
    Best,
    Worst,
}

#[derive(Clone, Debug)]
struct RunResult {
    /// Round-trip time in nanoseconds (write → response received).
    rtt_ns: u64,
    /// The server's stderr output — contains the profiler call-tree.
    trace: String,
    /// The RESP response rendered as a string.
    response: String,
    /// Run index (1-based, warmups excluded).
    index: usize,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.count == 0 {
        eprintln!("prof-cli: --count must be > 0");
        return ExitCode::FAILURE;
    }

    let argv = match parse_command(&args.command) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prof-cli: {e}");
            return ExitCode::FAILURE;
        }
    };

    if args.warmup > 0 {
        for _ in 0..args.warmup {
            if let Err(e) = warmup_run(&args.server_bin, &argv) {
                eprintln!("prof-cli: warmup failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    eprint_section("Profiling", args.count);

    let mut results: Vec<RunResult> = Vec::with_capacity(args.count);

    for i in 1..=args.count {
        match timed_run(&args.server_bin, &argv, args.plain, i) {
            Ok(r) => {
                if args.result_type == ResultType::All {
                    eprint!("{}", r.trace);
                    if !r.response.is_empty() {
                        eprintln!("  \x1b[2mresponse:\x1b[0m {}", r.response);
                    }
                }
                results.push(r);
            }
            Err(e) => {
                eprintln!("prof-cli: run #{i} failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    match args.result_type {
        ResultType::All => {
            render_summary(&results);
        }
        ResultType::Avg => {
            let chosen = pick_avg(&results);
            eprint!("{}", chosen.trace);
            if !chosen.response.is_empty() {
                eprintln!("  \x1b[2mresponse:\x1b[0m {}", chosen.response);
            }
            render_summary(&results);
        }
        ResultType::Best => {
            let chosen = results.iter().min_by_key(|r| r.rtt_ns).unwrap();
            eprint!("{}", chosen.trace);
            if !chosen.response.is_empty() {
                eprintln!("  \x1b[2mresponse:\x1b[0m {}", chosen.response);
            }
            render_summary(&results);
        }
        ResultType::Worst => {
            let chosen = results.iter().max_by_key(|r| r.rtt_ns).unwrap();
            eprint!("{}", chosen.trace);
            if !chosen.response.is_empty() {
                eprintln!("  \x1b[2mresponse:\x1b[0m {}", chosen.response);
            }
            render_summary(&results);
        }
    }

    ExitCode::SUCCESS
}

fn timed_run(
    server_bin: &str,
    argv: &[Vec<u8>],
    plain: bool,
    index: usize,
) -> Result<RunResult, String> {
    let port = find_free_port().ok_or("no free port available")?;
    let pretty = if plain { "0" } else { "1" };

    let mut child = std::process::Command::new(server_bin)
        .args(["--port", &port.to_string(), "--bind", "127.0.0.1"])
        .env("JUSTKV_TRACE", "1")
        .env("JUSTKV_TRACE_MAX", "1")
        .env("JUSTKV_PRETTY", pretty)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start '{server_bin}': {e}"))?;

    let stderr_handle = {
        let stderr = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            use std::io::Read;
            let mut buf = String::new();
            let mut reader = std::io::BufReader::new(stderr);
            let _ = reader.read_to_string(&mut buf);
            buf
        })
    };

    let result = run_in_rt(async move {
        wait_for_server(port, Duration::from_secs(5)).await?;
        send_one(port, argv).await
    });

    let _ = child.kill();
    let _ = child.wait();

    let trace = stderr_handle.join().unwrap_or_default();
    let (rtt_ns, response) = result?;

    Ok(RunResult {
        rtt_ns,
        trace,
        response,
        index,
    })
}

fn warmup_run(server_bin: &str, argv: &[Vec<u8>]) -> Result<(), String> {
    let port = find_free_port().ok_or("no free port available")?;

    let mut child = std::process::Command::new(server_bin)
        .args(["--port", &port.to_string(), "--bind", "127.0.0.1"])
        // No trace env vars — warmup results are discarded.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start '{server_bin}': {e}"))?;

    run_in_rt(async move {
        wait_for_server(port, Duration::from_secs(5)).await?;
        send_one(port, argv).await?;
        Ok(())
    })?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn run_in_rt<F, T>(fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(fut)
}

async fn wait_for_server(port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "server on :{port} did not become ready within {}s",
                timeout.as_secs()
            ));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn send_one(port: u16, argv: &[Vec<u8>]) -> Result<(u64, String), String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| format!("connect: {e}"))?;

    let frame = RespFrame::Array(Some(
        argv.iter()
            .map(|p| RespFrame::Bulk(Some(BulkData::from_vec(p.clone()))))
            .collect(),
    ));

    let mut out = BytesMut::with_capacity(256);
    encoder::encode(&frame, &mut out);

    let t0 = Instant::now();
    stream
        .write_all(&out)
        .await
        .map_err(|e| format!("write: {e}"))?;

    let resp = read_one_frame(&mut stream).await?;
    let rtt_ns = t0.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;

    Ok((rtt_ns, format_resp(&resp)))
}

async fn read_one_frame(stream: &mut TcpStream) -> Result<RespFrame, String> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parser::parse_frame(&mut buf) {
            Ok(Some(frame)) => return Ok(frame),
            Ok(None) | Err(ParseError::Incomplete) => {}
            Err(ParseError::Protocol(e)) => return Err(format!("protocol error: {e}")),
        }
        let mut chunk = [0u8; 4096];
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            return Err("connection closed".to_string());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn find_free_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    Some(listener.local_addr().ok()?.port())
}

fn parse_command(input: &str) -> Result<Vec<Vec<u8>>, String> {
    let parts = shlex_split(input)?;
    if parts.is_empty() {
        return Err("empty command".to_string());
    }
    Ok(parts.into_iter().map(String::into_bytes).collect())
}

fn shlex_split(input: &str) -> Result<Vec<String>, String> {
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_double = false;
    let mut in_single = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            ' ' | '\t' if !in_double && !in_single => {
                if !cur.is_empty() {
                    parts.push(cur.drain(..).collect());
                }
            }
            '\\' if in_double => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            _ => cur.push(c),
        }
    }
    if in_double || in_single {
        return Err("unclosed quote in command string".to_string());
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    Ok(parts)
}

fn format_resp(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(s) => s.clone(),
        RespFrame::Error(e) => format!("(error) {e}"),
        RespFrame::Integer(n) => format!("(integer) {n}"),
        RespFrame::Bulk(None) => "(nil)".to_string(),
        RespFrame::Bulk(Some(b)) => String::from_utf8_lossy(b.as_slice()).into_owned(),
        RespFrame::Array(None) => "(empty)".to_string(),
        RespFrame::Array(Some(items)) => items
            .iter()
            .enumerate()
            .map(|(i, f)| format!("{}) {}", i + 1, format_resp(f)))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "(other)".to_string(),
    }
}

fn pick_avg(results: &[RunResult]) -> &RunResult {
    let avg = results.iter().map(|r| r.rtt_ns).sum::<u64>() / results.len() as u64;
    results
        .iter()
        .min_by_key(|r| r.rtt_ns.abs_diff(avg))
        .unwrap()
}

pub fn fmt_time(us: f64) -> String {
    if us >= 1_000.0 {
        format!("{:.2}ms", us / 1_000.0)
    } else if us >= 1.0 {
        format!("{:.2}µs", us)
    } else {
        format!("{:.0}ns", us * 1_000.0)
    }
}

pub fn ns_to_us(ns: u64) -> f64 {
    ns as f64 / 1_000.0
}

fn render_summary(results: &[RunResult]) {
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";
    const DIM: &str = "\x1b[2m";
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const MAGENTA: &str = "\x1b[35m";
    const WHITE: &str = "\x1b[37m";

    const COL_AVG: usize = 9;
    const COL_MIN: usize = 9;
    const COL_MAX: usize = 9;
    const COL_RUNS: usize = 5;

    let n = results.len();
    if n == 0 {
        return;
    }

    let sum: u64 = results.iter().map(|r| r.rtt_ns).sum();
    let avg_ns = sum / n as u64;
    let min_ns = results.iter().map(|r| r.rtt_ns).min().unwrap();
    let max_ns = results.iter().map(|r| r.rtt_ns).max().unwrap();

    let avg_str = fmt_time(ns_to_us(avg_ns));
    let min_str = fmt_time(ns_to_us(min_ns));
    let max_str = fmt_time(ns_to_us(max_ns));

    let sec_avg = COL_AVG + 2;
    let sec_min = COL_MIN + 2;
    let sec_max = COL_MAX + 2;
    let sec_runs = COL_RUNS + 2;
    let inner_width = sec_avg + 1 + sec_min + 1 + sec_max + 1 + sec_runs;

    let pad = |s: &str, w: usize| -> String {
        let chars = s.chars().count();
        if chars >= w {
            s.to_string()
        } else {
            format!("{:>width$}", s, width = w)
        }
    };

    let full_row = |content: &str, visible_len: usize| {
        let pad_right = if visible_len < inner_width {
            inner_width - visible_len
        } else {
            0
        };
        eprintln!("{CYAN}║{RESET}{content}{:pad_right$}{CYAN}║{RESET}", "");
    };

    eprintln!();
    eprintln!("{BOLD}{CYAN}╔{}╗{RESET}", "═".repeat(inner_width));

    let title_content = format!(
        "  {BOLD}Summary{RESET}  {DIM}({n} run{s}){RESET}",
        s = if n == 1 { "" } else { "s" }
    );
    let title_visible = format!("  Summary  ({n} run{s})", s = if n == 1 { "" } else { "s" })
        .chars()
        .count();
    full_row(&title_content, title_visible);

    eprintln!(
        "{BOLD}{CYAN}╠{}╤{}╤{}╤{}╣{RESET}",
        "═".repeat(sec_avg),
        "═".repeat(sec_min),
        "═".repeat(sec_max),
        "═".repeat(sec_runs),
    );

    eprintln!(
        "{CYAN}║{RESET} {DIM}{avg_lbl:>COL_AVG$}{RESET} {CYAN}│{RESET} {DIM}{min_lbl:>COL_MIN$}{RESET} {CYAN}│{RESET} {DIM}{max_lbl:>COL_MAX$}{RESET} {CYAN}│{RESET} {DIM}{runs_lbl:>COL_RUNS$}{RESET} {CYAN}║{RESET}",
        avg_lbl = "avg",
        min_lbl = "min",
        max_lbl = "max",
        runs_lbl = "runs",
    );

    eprintln!(
        "{BOLD}{CYAN}╠{}╪{}╪{}╪{}╣{RESET}",
        "═".repeat(sec_avg),
        "═".repeat(sec_min),
        "═".repeat(sec_max),
        "═".repeat(sec_runs),
    );

    eprintln!(
        "{CYAN}║{RESET} {BOLD}{WHITE}{avg:>COL_AVG$}{RESET} {CYAN}│{RESET} {GREEN}{min:>COL_MIN$}{RESET} {CYAN}│{RESET} {MAGENTA}{max:>COL_MAX$}{RESET} {CYAN}│{RESET} {DIM}{n:>COL_RUNS$}{RESET} {CYAN}║{RESET}",
        avg = pad(&avg_str, COL_AVG),
        min = pad(&min_str, COL_MIN),
        max = pad(&max_str, COL_MAX),
        MAGENTA = MAGENTA,
    );

    if n > 1 {
        let max_idx_width = results
            .iter()
            .map(|r| format!("#{}", r.index).chars().count())
            .max()
            .unwrap_or(2);
        const COL_RTT: usize = COL_AVG;

        eprintln!(
            "{BOLD}{CYAN}╠{}╧{}╧{}╧{}╣{RESET}",
            "═".repeat(sec_avg),
            "═".repeat(sec_min),
            "═".repeat(sec_max),
            "═".repeat(sec_runs),
        );

        let sub_lbl = format!(" {DIM}run{RESET}  {DIM}RTT{RESET}  {DIM}bar{RESET}");
        let sub_lbl_visible = " run  RTT  bar".chars().count();
        full_row(&sub_lbl, sub_lbl_visible);

        eprintln!("{BOLD}{CYAN}╠{}╣{RESET}", "═".repeat(inner_width));

        for r in results {
            let t = fmt_time(ns_to_us(r.rtt_ns));
            let pct = if max_ns == 0 {
                0.0
            } else {
                r.rtt_ns as f64 / max_ns as f64 * 100.0
            };
            let filled = ((pct / 100.0) * 20.0).round() as usize;
            let filled = filled.min(20);
            let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(20 - filled),);
            let idx_str = format!("#{}", r.index);

            let content_visible = format!(
                " {idx_str:<max_idx_width$}  {rtt:>COL_RTT$}   {bar}",
                rtt = pad(&t, COL_RTT),
            );
            let visible_len = content_visible.chars().count();

            let content = format!(
                " {BOLD}{idx_str:<max_idx_width$}{RESET}  {BOLD}{rtt:>COL_RTT$}{RESET}   {DIM}{bar}{RESET}",
                rtt = pad(&t, COL_RTT),
            );
            full_row(&content, visible_len);
        }
    }

    eprintln!("{BOLD}{CYAN}╚{}╝{RESET}", "═".repeat(inner_width));
    eprintln!();
}

fn eprint_section(label: &str, n: usize) {
    eprintln!(
        "\x1b[1m\x1b[36m── {label} ({n} run{s}) ──\x1b[0m",
        s = if n == 1 { "" } else { "s" }
    );
}

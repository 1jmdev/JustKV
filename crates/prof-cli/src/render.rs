use crate::cli::ResultType;
use crate::session::RunResult;

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

pub fn pick_avg(results: &[RunResult]) -> &RunResult {
    let avg = results.iter().map(|r| r.rtt_ns).sum::<u64>() / results.len() as u64;
    results
        .iter()
        .min_by_key(|run| run.rtt_ns.abs_diff(avg))
        .expect("non-empty results")
}

pub fn eprint_section(label: &str, n: usize) {
    eprintln!(
        "\x1b[1m\x1b[36m── {label} ({n} run{s}) ──\x1b[0m",
        s = if n == 1 { "" } else { "s" }
    );
}

pub fn render_responses(results: &[RunResult], result_type: &ResultType) {
    if results.is_empty() {
        return;
    }
    match result_type {
        ResultType::All => {
            for run in results {
                if !run.response.is_empty() {
                    eprintln!("  \x1b[2mresponse #{}:\x1b[0m {}", run.index, run.response);
                }
            }
        }
        ResultType::Avg => {
            let run = pick_avg(results);
            if !run.response.is_empty() {
                eprintln!("  \x1b[2mresponse #{}:\x1b[0m {}", run.index, run.response);
            }
        }
        ResultType::Best => {
            if let Some(run) = results.iter().min_by_key(|r| r.rtt_ns) {
                if !run.response.is_empty() {
                    eprintln!("  \x1b[2mresponse #{}:\x1b[0m {}", run.index, run.response);
                }
            }
        }
        ResultType::Worst => {
            if let Some(run) = results.iter().max_by_key(|r| r.rtt_ns) {
                if !run.response.is_empty() {
                    eprintln!("  \x1b[2mresponse #{}:\x1b[0m {}", run.index, run.response);
                }
            }
        }
    }
}

pub fn render_summary(results: &[RunResult]) {
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
    let min_ns = results.iter().map(|r| r.rtt_ns).min().unwrap_or(0);
    let max_ns = results.iter().map(|r| r.rtt_ns).max().unwrap_or(0);

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
        let pad_right = inner_width.saturating_sub(visible_len);
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
            let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(20 - filled));
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

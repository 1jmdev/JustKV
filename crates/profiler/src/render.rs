use crate::trace::{ActiveTrace, TraceResult};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";
const WHITE: &str = "\x1b[37m";

const COL_TOTAL: usize = 9;
const COL_SELF: usize = 9;
const COL_BAR: usize = 12;
const COL_PCT: usize = 6;
const STATS_PAD: usize = 4;

struct Layout {
    name_col: usize,
    sec_name: usize,
    sec_total: usize,
    sec_self: usize,
    sec_bar: usize,
    sec_pct: usize,
    inner_width: usize,
}

impl Layout {
    fn new(name_col: usize) -> Self {
        let sec_name = name_col + 3;
        let sec_total = COL_TOTAL + 2;
        let sec_self = COL_SELF + 2;
        let sec_bar = COL_BAR + 2;
        let sec_pct = COL_PCT + 2;
        let inner_width = sec_name + 1 + sec_total + 1 + sec_self + 1 + sec_bar + 1 + sec_pct;
        Self {
            name_col,
            sec_name,
            sec_total,
            sec_self,
            sec_bar,
            sec_pct,
            inner_width,
        }
    }

    fn top_border(&self) -> String {
        format!("{BOLD}{CYAN}╔{}╗{RESET}", "═".repeat(self.inner_width),)
    }

    fn header_separator(&self) -> String {
        format!(
            "{BOLD}{CYAN}╠{}╤{}╤{}╤{}╤{}╣{RESET}",
            "═".repeat(self.sec_name),
            "═".repeat(self.sec_total),
            "═".repeat(self.sec_self),
            "═".repeat(self.sec_bar),
            "═".repeat(self.sec_pct),
        )
    }

    fn column_separator(&self) -> String {
        format!(
            "{BOLD}{CYAN}╠{}╪{}╪{}╪{}╪{}╣{RESET}",
            "═".repeat(self.sec_name),
            "═".repeat(self.sec_total),
            "═".repeat(self.sec_self),
            "═".repeat(self.sec_bar),
            "═".repeat(self.sec_pct),
        )
    }

    fn bottom_border(&self) -> String {
        format!(
            "{BOLD}{CYAN}╚{}╧{}╧{}╧{}╧{}╝{RESET}",
            "═".repeat(self.sec_name),
            "═".repeat(self.sec_total),
            "═".repeat(self.sec_self),
            "═".repeat(self.sec_bar),
            "═".repeat(self.sec_pct),
        )
    }

    fn full_row(&self, content: &str, visible_len: usize) -> String {
        let pad = if visible_len < self.inner_width {
            self.inner_width - visible_len
        } else {
            0
        };
        format!("{CYAN}║{RESET}{content}{:pad$}{CYAN}║{RESET}", "")
    }
}

pub(crate) fn render_trace(trace: &ActiveTrace) {
    if !trace.pretty {
        render_trace_plain(trace);
        return;
    }

    let root = &trace.nodes[0];
    let command = String::from_utf8_lossy(&trace.command);
    let key = trace
        .key
        .as_ref()
        .map(|k| String::from_utf8_lossy(k).into_owned())
        .unwrap_or_else(|| "-".into());

    let total_str = fmt_time(ns_to_us(root.total_ns));
    let self_str = fmt_time(ns_to_us(root.self_ns));

    let mut max_name: usize = 0;
    for (idx, child) in root.children.iter().enumerate() {
        let is_last = idx + 1 == root.children.len();
        measure_max_name_active(trace, *child, "", is_last, &mut max_name);
    }
    let layout = Layout::new(max_name + STATS_PAD);

    eprintln!();
    eprintln!("{}", layout.top_border());

    let cmd_content = format!("  {BOLD}{command}{RESET}  {DIM}key={key}{RESET}");
    let cmd_visible = format!("  {command}  key={key}").chars().count();
    eprintln!("{}", layout.full_row(&cmd_content, cmd_visible));

    let t = pad_left(&total_str, COL_TOTAL);
    let s = pad_left(&self_str, COL_SELF);
    let totals_content =
        format!("  {DIM}total{RESET} {BOLD}{WHITE}{t}{RESET}   {DIM}self{RESET} {DIM}{s}{RESET}");
    let totals_visible = format!("  total {t}   self {s}").chars().count();
    eprintln!("{}", layout.full_row(&totals_content, totals_visible));

    eprintln!("{}", layout.header_separator());

    eprintln!(
        "{CYAN}║{RESET} {DIM}{:<name_col$}{RESET}  {CYAN}│{RESET} {DIM}{:>COL_TOTAL$}{RESET} {CYAN}│{RESET} {DIM}{:>COL_SELF$}{RESET} {CYAN}│{RESET} {DIM}{:<COL_BAR$}{RESET} {CYAN}│{RESET} {DIM}{:>COL_PCT$}{RESET} {CYAN}║{RESET}",
        "function",
        "total",
        "self",
        "",
        "%",
        name_col = layout.name_col,
    );

    eprintln!("{}", layout.column_separator());

    for (idx, child) in root.children.iter().enumerate() {
        let is_last = idx + 1 == root.children.len();
        render_node_active(trace, *child, "", is_last, root.total_ns, &layout);
    }

    eprintln!("{}", layout.bottom_border());
    eprintln!();
}

fn render_trace_plain(trace: &ActiveTrace) {
    let root = &trace.nodes[0];
    let command = String::from_utf8_lossy(&trace.command);
    let key = trace
        .key
        .as_ref()
        .map(|k| String::from_utf8_lossy(k).into_owned())
        .unwrap_or_else(|| "-".into());

    eprintln!(
        "TRACE command={} key={} total={} self={}",
        command,
        key,
        fmt_time(ns_to_us(root.total_ns)),
        fmt_time(ns_to_us(root.self_ns)),
    );
    eprintln!("scope\tdepth\ttotal\tself\tpct\tname");

    for child in &root.children {
        render_plain_node_active(trace, *child, 0, root.total_ns);
    }
    eprintln!();
}

fn measure_max_name_active(
    trace: &ActiveTrace,
    node_index: usize,
    prefix: &str,
    is_last: bool,
    max: &mut usize,
) {
    let node = &trace.nodes[node_index];
    let connector = if is_last { "└─ " } else { "├─ " };
    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

    let short = shorten(node.name);
    let (module, func) = split_name(&short);
    let visible_len = prefix.chars().count()
        + connector.chars().count()
        + module.chars().count()
        + 2
        + func.chars().count();

    if visible_len > *max {
        *max = visible_len;
    }

    for (idx, child) in node.children.iter().enumerate() {
        let child_last = idx + 1 == node.children.len();
        measure_max_name_active(trace, *child, &child_prefix, child_last, max);
    }
}

fn render_node_active(
    trace: &ActiveTrace,
    node_index: usize,
    prefix: &str,
    is_last: bool,
    root_total_ns: u64,
    layout: &Layout,
) {
    let node = &trace.nodes[node_index];

    let connector = if is_last { "└─ " } else { "├─ " };
    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

    let total_us = ns_to_us(node.total_ns);
    let self_us = ns_to_us(node.self_ns);
    let pct = if root_total_ns == 0 {
        0.0
    } else {
        node.total_ns as f64 / root_total_ns as f64 * 100.0
    };

    let short = shorten(node.name);
    let (module, func) = split_name(&short);

    let total_str = fmt_time(total_us);
    let self_str = fmt_time(self_us);
    let bar = pct_bar(pct);
    let pct_str = format!("{:.1}%", pct);
    let time_col = time_color(total_us);
    let self_col = time_color(self_us);

    let name_visible = format!("{prefix}{connector}{module}::{func}");
    let name_len = name_visible.chars().count();
    let gap = if name_len < layout.name_col {
        layout.name_col - name_len
    } else {
        1
    };

    eprintln!(
        "{CYAN}║{RESET} {CYAN}{prefix}{connector}{RESET}{DIM}{module}::{RESET}{BOLD}{func}{RESET}{:gap$}  {CYAN}│{RESET} {time_col}{BOLD}{}{RESET} {CYAN}│{RESET} {self_col}{BOLD}{}{RESET} {CYAN}│{RESET} {DIM}{bar}{RESET} {CYAN}│{RESET} {DIM}{}{RESET} {CYAN}║{RESET}",
        "",
        pad_left(&total_str, COL_TOTAL),
        pad_left(&self_str, COL_SELF),
        pad_left(&pct_str, COL_PCT),
    );

    for (idx, child) in node.children.iter().enumerate() {
        let child_last = idx + 1 == node.children.len();
        render_node_active(
            trace,
            *child,
            &child_prefix,
            child_last,
            root_total_ns,
            layout,
        );
    }
}

fn render_plain_node_active(
    trace: &ActiveTrace,
    node_index: usize,
    depth: usize,
    root_total_ns: u64,
) {
    let node = &trace.nodes[node_index];

    let total_us = ns_to_us(node.total_ns);
    let self_us = ns_to_us(node.self_ns);
    let pct = if root_total_ns == 0 {
        0.0
    } else {
        node.total_ns as f64 / root_total_ns as f64 * 100.0
    };

    eprintln!(
        "scope\t{}\t{}\t{}\t{:.1}%\t{}",
        depth,
        fmt_time(total_us),
        fmt_time(self_us),
        pct,
        shorten(node.name),
    );

    for child in &node.children {
        render_plain_node_active(trace, *child, depth + 1, root_total_ns);
    }
}

pub fn render_result_pretty(result: &TraceResult) {
    let root = &result.nodes[0];
    let command = &result.command;
    let key = result.key.as_deref().unwrap_or("-");

    let total_str = fmt_time(ns_to_us(root.total_ns));
    let self_str = fmt_time(ns_to_us(root.self_ns));

    let mut max_name: usize = 0;
    for (idx, child) in root.children.iter().enumerate() {
        let is_last = idx + 1 == root.children.len();
        measure_max_name_result(result, *child, "", is_last, &mut max_name);
    }
    let layout = Layout::new(max_name + STATS_PAD);

    eprintln!();
    eprintln!("{}", layout.top_border());

    let cmd_content = format!("  {BOLD}{command}{RESET}  {DIM}key={key}{RESET}");
    let cmd_visible = format!("  {command}  key={key}").chars().count();
    eprintln!("{}", layout.full_row(&cmd_content, cmd_visible));

    let t = pad_left(&total_str, COL_TOTAL);
    let s = pad_left(&self_str, COL_SELF);
    let totals_content =
        format!("  {DIM}total{RESET} {BOLD}{WHITE}{t}{RESET}   {DIM}self{RESET} {DIM}{s}{RESET}");
    let totals_visible = format!("  total {t}   self {s}").chars().count();
    eprintln!("{}", layout.full_row(&totals_content, totals_visible));

    eprintln!("{}", layout.header_separator());

    eprintln!(
        "{CYAN}║{RESET} {DIM}{:<name_col$}{RESET}  {CYAN}│{RESET} {DIM}{:>COL_TOTAL$}{RESET} {CYAN}│{RESET} {DIM}{:>COL_SELF$}{RESET} {CYAN}│{RESET} {DIM}{:<COL_BAR$}{RESET} {CYAN}│{RESET} {DIM}{:>COL_PCT$}{RESET} {CYAN}║{RESET}",
        "function",
        "total",
        "self",
        "",
        "%",
        name_col = layout.name_col,
    );

    eprintln!("{}", layout.column_separator());

    for (idx, child) in root.children.iter().enumerate() {
        let is_last = idx + 1 == root.children.len();
        render_node_result(result, *child, "", is_last, root.total_ns, &layout);
    }

    eprintln!("{}", layout.bottom_border());
    eprintln!();
}

pub fn render_result_plain(result: &TraceResult) {
    let root = &result.nodes[0];
    let command = &result.command;
    let key = result.key.as_deref().unwrap_or("-");

    eprintln!(
        "TRACE command={} key={} total={} self={}",
        command,
        key,
        fmt_time(ns_to_us(root.total_ns)),
        fmt_time(ns_to_us(root.self_ns)),
    );
    eprintln!("scope\tdepth\ttotal\tself\tpct\tname");

    for child in &root.children {
        render_plain_node_result(result, *child, 0, root.total_ns);
    }
    eprintln!();
}

fn measure_max_name_result(
    result: &TraceResult,
    node_index: usize,
    prefix: &str,
    is_last: bool,
    max: &mut usize,
) {
    let node = &result.nodes[node_index];
    let connector = if is_last { "└─ " } else { "├─ " };
    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

    let short = shorten(node.name);
    let (module, func) = split_name(&short);
    let visible_len = prefix.chars().count()
        + connector.chars().count()
        + module.chars().count()
        + 2
        + func.chars().count();

    if visible_len > *max {
        *max = visible_len;
    }

    for (idx, child) in node.children.iter().enumerate() {
        let child_last = idx + 1 == node.children.len();
        measure_max_name_result(result, *child, &child_prefix, child_last, max);
    }
}

fn render_node_result(
    result: &TraceResult,
    node_index: usize,
    prefix: &str,
    is_last: bool,
    root_total_ns: u64,
    layout: &Layout,
) {
    let node = &result.nodes[node_index];

    let connector = if is_last { "└─ " } else { "├─ " };
    let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

    let total_us = ns_to_us(node.total_ns);
    let self_us = ns_to_us(node.self_ns);
    let pct = if root_total_ns == 0 {
        0.0
    } else {
        node.total_ns as f64 / root_total_ns as f64 * 100.0
    };

    let short = shorten(node.name);
    let (module, func) = split_name(&short);

    let total_str = fmt_time(total_us);
    let self_str = fmt_time(self_us);
    let bar = pct_bar(pct);
    let pct_str = format!("{:.1}%", pct);
    let time_col = time_color(total_us);
    let self_col = time_color(self_us);

    let name_visible = format!("{prefix}{connector}{module}::{func}");
    let name_len = name_visible.chars().count();
    let gap = if name_len < layout.name_col {
        layout.name_col - name_len
    } else {
        1
    };

    eprintln!(
        "{CYAN}║{RESET} {CYAN}{prefix}{connector}{RESET}{DIM}{module}::{RESET}{BOLD}{func}{RESET}{:gap$}  {CYAN}│{RESET} {time_col}{BOLD}{}{RESET} {CYAN}│{RESET} {self_col}{BOLD}{}{RESET} {CYAN}│{RESET} {DIM}{bar}{RESET} {CYAN}│{RESET} {DIM}{}{RESET} {CYAN}║{RESET}",
        "",
        pad_left(&total_str, COL_TOTAL),
        pad_left(&self_str, COL_SELF),
        pad_left(&pct_str, COL_PCT),
    );

    for (idx, child) in node.children.iter().enumerate() {
        let child_last = idx + 1 == node.children.len();
        render_node_result(
            result,
            *child,
            &child_prefix,
            child_last,
            root_total_ns,
            layout,
        );
    }
}

fn render_plain_node_result(
    result: &TraceResult,
    node_index: usize,
    depth: usize,
    root_total_ns: u64,
) {
    let node = &result.nodes[node_index];

    let total_us = ns_to_us(node.total_ns);
    let self_us = ns_to_us(node.self_ns);
    let pct = if root_total_ns == 0 {
        0.0
    } else {
        node.total_ns as f64 / root_total_ns as f64 * 100.0
    };

    eprintln!(
        "scope\t{}\t{}\t{}\t{:.1}%\t{}",
        depth,
        fmt_time(total_us),
        fmt_time(self_us),
        pct,
        shorten(node.name),
    );

    for child in &node.children {
        render_plain_node_result(result, *child, depth + 1, root_total_ns);
    }
}

fn shorten(name: &str) -> String {
    name.split("::")
        .filter(|s| *s != "crates" && *s != "src")
        .collect::<Vec<_>>()
        .join("::")
}

fn split_name(name: &str) -> (&str, &str) {
    match name.rfind("::") {
        Some(pos) => (&name[..pos], &name[pos + 2..]),
        None => ("", name),
    }
}

fn pad_left(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{:>width$}", s, width = width)
    }
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

fn time_color(us: f64) -> &'static str {
    if us >= 100.0 {
        MAGENTA
    } else if us >= 10.0 {
        YELLOW
    } else if us >= 1.0 {
        GREEN
    } else {
        BLUE
    }
}

fn pct_bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * 10.0).round() as usize;
    let filled = filled.min(10);
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

pub fn ns_to_us(ns: u64) -> f64 {
    ns as f64 / 1_000.0
}

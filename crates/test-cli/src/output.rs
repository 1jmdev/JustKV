use indicatif::{ProgressBar, ProgressStyle};
use protocol::types::{BulkData, RespFrame};

use crate::model::{RunSummary, TestFailure};

pub struct Ui {
    progress: ProgressBar,
    quiet: bool,
}

impl Ui {
    pub fn new(total: usize, quiet: bool) -> Self {
        let progress = if quiet {
            ProgressBar::hidden()
        } else {
            let progress = ProgressBar::new(total as u64);
            progress.set_style(
                ProgressStyle::with_template(
                    "{spinner:.cyan} [{elapsed_precise}] [{bar:32.cyan/blue}] {pos:>3}/{len:3} {msg}",
                )
                .unwrap()
                .progress_chars("=>-"),
            );
            progress.enable_steady_tick(std::time::Duration::from_millis(80));
            progress
        };

        Self { progress, quiet }
    }

    pub fn set_discovery(&self, files: usize, tests: usize) {
        if !self.quiet {
            self.progress
                .set_message(format!("discovered {files} file(s), {tests} test(s)"));
        }
    }

    pub fn set_current_test(&self, location: &str) {
        if !self.quiet {
            self.progress.set_message(location.to_string());
        }
    }

    pub fn record_success(&self, location: &str) {
        self.progress.inc(1);
        if !self.quiet {
            self.progress.set_message(format!("ok {location}"));
        }
    }

    pub fn record_failure(&self, location: &str) {
        self.progress.inc(1);
        if !self.quiet {
            self.progress.set_message(format!("failed {location}"));
        }
    }

    pub fn finish(&self, summary: &RunSummary) {
        if self.quiet {
            return;
        }

        let status = if summary.failed == 0 {
            if summary.skipped == 0 {
                format!("done: {} passed", summary.passed)
            } else {
                format!(
                    "done: {} passed, {} skipped",
                    summary.passed, summary.skipped
                )
            }
        } else {
            format!(
                "done: {} passed, {} skipped, {} failed",
                summary.passed, summary.skipped, summary.failed
            )
        };
        self.progress.set_message(status);
        self.progress.finish_and_clear();
    }
}

pub fn print_failures(failures: &[TestFailure]) {
    if failures.is_empty() {
        return;
    }

    println!();
    println!("Failures:");
    for failure in failures {
        println!(
            "- {} :: {} ({:.2} ms)",
            failure.path.display(),
            failure.test_name,
            failure.elapsed.as_secs_f64() * 1000.0
        );
        println!("  {}", failure.error.replace('\n', "\n  "));
    }
}

pub fn print_warnings(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    println!();
    println!("Warnings:");
    for warning in warnings {
        println!("- {warning}");
    }
}

pub fn print_summary(summary: &RunSummary) {
    println!();
    println!(
        "Result: {} discovered, {} executed, {} passed, {} skipped, {} failed in {:.2} ms",
        summary.discovered_total,
        summary.total,
        summary.passed,
        summary.skipped,
        summary.failed,
        summary.elapsed.as_secs_f64() * 1000.0
    );
}

pub fn render_frame(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(value) => value.clone(),
        RespFrame::SimpleStatic(value) => (*value).to_string(),
        RespFrame::Error(value) => format!("(error) {value}"),
        RespFrame::ErrorStatic(value) => format!("(error) {value}"),
        RespFrame::Integer(value) => format!("(integer) {value}"),
        RespFrame::Bulk(None) => "(nil)".to_string(),
        RespFrame::Bulk(Some(BulkData::Arg(value))) => quote_bytes(value.as_slice()),
        RespFrame::Bulk(Some(BulkData::Value(value))) => quote_bytes(value.as_slice()),
        RespFrame::Array(None) => "(nil)".to_string(),
        RespFrame::Array(Some(items)) if items.is_empty() => "(empty array)".to_string(),
        RespFrame::Array(Some(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}) {}", index + 1, render_frame(item)))
            .collect::<Vec<_>>()
            .join("\n"),
        RespFrame::BulkOptions(items) => {
            if items.is_empty() {
                return "(empty array)".to_string();
            }
            items
                .iter()
                .enumerate()
                .map(|(index, item)| match item {
                    Some(value) => format!("{}) {}", index + 1, quote_bytes(value.as_slice())),
                    None => format!("{}) (nil)", index + 1),
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        RespFrame::BulkValues(items) => {
            if items.is_empty() {
                return "(empty array)".to_string();
            }
            items
                .iter()
                .enumerate()
                .map(|(index, item)| format!("{}) {}", index + 1, quote_bytes(item.as_slice())))
                .collect::<Vec<_>>()
                .join("\n")
        }
        RespFrame::Map(items) => {
            if items.is_empty() {
                return "(empty map)".to_string();
            }
            items
                .iter()
                .enumerate()
                .map(|(index, (key, value))| {
                    format!(
                        "{}) {} => {}",
                        index + 1,
                        render_frame(key),
                        render_frame(value)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        RespFrame::PreEncoded(_) => "(pre-encoded frame)".to_string(),
    }
}

pub fn render_frame_raw(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(value) => value.clone(),
        RespFrame::SimpleStatic(value) => (*value).to_string(),
        RespFrame::Error(value) => value.clone(),
        RespFrame::ErrorStatic(value) => (*value).to_string(),
        RespFrame::Integer(value) => value.to_string(),
        RespFrame::Bulk(None) => "".to_string(),
        RespFrame::Bulk(Some(BulkData::Arg(value))) => {
            String::from_utf8_lossy(value.as_slice()).into_owned()
        }
        RespFrame::Bulk(Some(BulkData::Value(value))) => {
            String::from_utf8_lossy(value.as_slice()).into_owned()
        }
        RespFrame::Array(None) => "".to_string(),
        RespFrame::Array(Some(items)) => items
            .iter()
            .map(render_frame_raw)
            .collect::<Vec<_>>()
            .join("\n"),
        RespFrame::BulkOptions(items) => items
            .iter()
            .map(|item| match item {
                Some(value) => String::from_utf8_lossy(value.as_slice()).into_owned(),
                None => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        RespFrame::BulkValues(items) => items
            .iter()
            .map(|item| String::from_utf8_lossy(item.as_slice()).into_owned())
            .collect::<Vec<_>>()
            .join("\n"),
        RespFrame::Map(items) => items
            .iter()
            .map(|(key, value)| format!("{} {}", render_frame_raw(key), render_frame_raw(value)))
            .collect::<Vec<_>>()
            .join("\n"),
        RespFrame::PreEncoded(bytes) => String::from_utf8_lossy(bytes.as_ref()).into_owned(),
    }
}

fn quote_bytes(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    for byte in bytes {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            value if value.is_ascii_graphic() || *value == b' ' => out.push(*value as char),
            value => out.push_str(&format!("\\x{value:02x}")),
        }
    }
    out.push('"');
    out
}

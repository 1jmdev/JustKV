use std::time::Instant;

use protocol::types::{BulkData, RespFrame};

use crate::args::Args;
use crate::client::Client;
use crate::discovery::discover_rtest_files;
use crate::model::{ExpectedValue, RunSummary, TestFailure};
use crate::output::{Ui, print_failures, render_frame, render_frame_raw};
use crate::parser::parse_test_file;

pub async fn run(args: Args) -> Result<RunSummary, String> {
    let started = Instant::now();
    let paths = discover_rtest_files(&args.path)?;
    if paths.is_empty() {
        return Err(format!(
            "no .rtest files found under {}",
            args.path.display()
        ));
    }

    let files = paths
        .iter()
        .map(|path| parse_test_file(path))
        .collect::<Result<Vec<_>, _>>()?;

    let total = files.iter().map(|file| file.cases.len()).sum();
    let ui = Ui::new(total, args.quiet);
    ui.set_discovery(files.len(), total);

    let mut failures = Vec::new();
    let mut passed = 0usize;

    for file in files {
        for case in file.cases {
            let location = match &file.metadata.name {
                Some(name) => format!("{} :: {} :: {}", file.path.display(), name, case.name),
                None => format!("{} :: {}", file.path.display(), case.name),
            };
            ui.set_current_test(&location);

            let case_started = Instant::now();
            let result = run_case(&args, &case).await;
            let elapsed = case_started.elapsed();

            match result {
                Ok(()) => {
                    passed += 1;
                    ui.record_success(&location);
                }
                Err(error) => {
                    let failure = TestFailure {
                        path: file.path.clone(),
                        test_name: case.name,
                        elapsed,
                        error,
                    };
                    ui.record_failure(&location);
                    failures.push(failure);
                }
            }
        }
    }

    let summary = RunSummary {
        total,
        passed,
        failed: failures.len(),
        elapsed: started.elapsed(),
        failures,
    };

    ui.finish(&summary);
    print_failures(&summary.failures);

    Ok(summary)
}

async fn run_case(args: &Args, case: &crate::model::TestCase) -> Result<(), String> {
    let mut client = Client::connect(args).await?;
    client
        .flush_all()
        .await
        .map_err(|err| format!("FLUSHALL failed: {err}"))?;
    let mut captures = std::collections::HashMap::<String, String>::new();

    for command in &case.setup {
        let command = substitute_captures(command, &captures)?;
        client
            .execute_raw(&command)
            .await
            .map_err(|err| format!("setup command `{command}` failed: {err}"))?;
    }

    let no_reply = matches!(case.expect, ExpectedValue::NoReply);
    let mut responses = Vec::with_capacity(case.run.len());
    let last_index = case.run.len().saturating_sub(1);
    for (index, command) in case.run.iter().enumerate() {
        let command = substitute_captures(command, &captures)?;
        if no_reply && index == last_index {
            client
                .execute_raw_no_reply(&command)
                .await
                .map_err(|err| format!("run command `{command}` failed: {err}"))?;
        } else {
            let frame = client
                .execute_raw(&command)
                .await
                .map_err(|err| format!("run command `{command}` failed: {err}"))?;
            responses.push(frame);
        }
    }

    if !no_reply {
        validate_run_results(&case.expect, &responses, &mut captures)?;
    }

    for command in &case.cleanup {
        let command = substitute_captures(command, &captures)?;
        client
            .execute_raw(&command)
            .await
            .map_err(|err| format!("cleanup command `{command}` failed: {err}"))?;
    }

    Ok(())
}

fn validate_run_results(
    expected: &ExpectedValue,
    actual: &[RespFrame],
    captures: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    match expected {
        ExpectedValue::Sequence(items) => validate_sequence(items, actual, captures),
        _ => {
            let response = actual
                .last()
                .ok_or_else(|| "RUN section did not execute any command".to_string())?;
            validate_expected(expected, response, captures)
        }
    }
}

fn validate_sequence(
    expected: &[ExpectedValue],
    actual: &[RespFrame],
    captures: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if expected.len() != actual.len() {
        return Err(format!(
            "expected {} response(s), got {}",
            expected.len(),
            actual.len()
        ));
    }

    for (expected_item, actual_item) in expected.iter().zip(actual.iter()) {
        validate_expected(expected_item, actual_item, captures)?;
    }

    Ok(())
}

fn validate_expected(
    expected: &ExpectedValue,
    actual: &RespFrame,
    captures: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    match expected {
        ExpectedValue::Any => Ok(()),
        ExpectedValue::NoReply => Ok(()),
        ExpectedValue::Sequence(_) => {
            Err("internal error: sequence cannot validate a single response".to_string())
        }
        ExpectedValue::Capture(name) => {
            captures.insert(name.clone(), render_frame_raw(actual));
            Ok(())
        }
        ExpectedValue::Simple(value) => match actual {
            RespFrame::Simple(actual) if actual == value => Ok(()),
            RespFrame::SimpleStatic(actual) if actual == value => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::Bulk(None) => match actual {
            RespFrame::Bulk(None) => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::Bulk(Some(value)) => match actual {
            RespFrame::Bulk(Some(BulkData::Arg(actual)))
                if actual.as_slice() == value.as_slice() =>
            {
                Ok(())
            }
            RespFrame::Bulk(Some(BulkData::Value(actual)))
                if actual.as_slice() == value.as_slice() =>
            {
                Ok(())
            }
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::IntegerAny => match actual {
            RespFrame::Integer(_) => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::Integer(value) => match actual {
            RespFrame::Integer(actual) if actual == value => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::ErrorAny => match actual {
            RespFrame::Error(_) | RespFrame::ErrorStatic(_) => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::ErrorPrefix(prefix) => match actual {
            RespFrame::Error(actual) if actual.starts_with(prefix) => Ok(()),
            RespFrame::ErrorStatic(actual) if actual.starts_with(prefix) => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::EmptyArray => match actual {
            RespFrame::Array(Some(items)) if items.is_empty() => Ok(()),
            RespFrame::BulkOptions(items) if items.is_empty() => Ok(()),
            RespFrame::BulkValues(items) if items.is_empty() => Ok(()),
            _ => Err(mismatch(expected, actual)),
        },
        ExpectedValue::RawRegex(regex) => {
            let rendered = render_frame_raw(actual);
            if regex.is_match(&rendered) {
                Ok(())
            } else {
                Err(format!(
                    "expected raw response to match `{}`, got:\n{}",
                    regex.as_str(),
                    render_frame(actual)
                ))
            }
        }
        ExpectedValue::Array { items, unordered } => {
            validate_array(items, *unordered, actual, captures)
        }
    }
}

fn validate_array(
    items: &[ExpectedValue],
    unordered: bool,
    actual: &RespFrame,
    captures: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let normalized;
    let actual_items = match actual {
        RespFrame::Array(Some(items)) => items,
        RespFrame::BulkOptions(items) => {
            normalized = items
                .iter()
                .map(|item| match item {
                    Some(value) => RespFrame::Bulk(Some(BulkData::Value(value.clone()))),
                    None => RespFrame::Bulk(None),
                })
                .collect::<Vec<_>>();
            &normalized
        }
        RespFrame::BulkValues(items) => {
            normalized = items
                .iter()
                .cloned()
                .map(BulkData::Value)
                .map(|value| RespFrame::Bulk(Some(value)))
                .collect::<Vec<_>>();
            &normalized
        }
        _ => {
            return Err(mismatch(
                &ExpectedValue::Array {
                    items: items.to_vec(),
                    unordered,
                },
                actual,
            ));
        }
    };

    if actual_items.len() != items.len() {
        return Err(format!(
            "expected {} array item(s), got {}:\n{}",
            items.len(),
            actual_items.len(),
            render_frame(actual)
        ));
    }

    if !unordered {
        for (expected_item, actual_item) in items.iter().zip(actual_items.iter()) {
            validate_expected(expected_item, actual_item, captures)?;
        }
        return Ok(());
    }

    let mut used = vec![false; actual_items.len()];
    for expected_item in items {
        let mut matched = false;
        for (index, actual_item) in actual_items.iter().enumerate() {
            if used[index] {
                continue;
            }
            let mut scratch = captures.clone();
            if validate_expected(expected_item, actual_item, &mut scratch).is_ok() {
                used[index] = true;
                *captures = scratch;
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(format!(
                "could not match unordered item `{}` in:\n{}",
                expected_to_string(expected_item),
                render_frame(actual)
            ));
        }
    }

    Ok(())
}

fn mismatch(expected: &ExpectedValue, actual: &RespFrame) -> String {
    format!(
        "expected {}, got:\n{}",
        expected_to_string(expected),
        render_frame(actual)
    )
}

fn expected_to_string(expected: &ExpectedValue) -> String {
    match expected {
        ExpectedValue::Any => "(any)".to_string(),
        ExpectedValue::NoReply => "(noreply)".to_string(),
        ExpectedValue::Sequence(items) => format!("sequence[{}]", items.len()),
        ExpectedValue::Capture(name) => format!("(capture) {name}"),
        ExpectedValue::Simple(value) => value.clone(),
        ExpectedValue::Bulk(None) => "(nil)".to_string(),
        ExpectedValue::Bulk(Some(value)) => render_expected_bytes(value),
        ExpectedValue::IntegerAny => "(integer) (any)".to_string(),
        ExpectedValue::Integer(value) => format!("(integer) {value}"),
        ExpectedValue::ErrorAny => "(error)".to_string(),
        ExpectedValue::ErrorPrefix(prefix) => format!("(error) {prefix}"),
        ExpectedValue::EmptyArray => "(empty array)".to_string(),
        ExpectedValue::RawRegex(regex) => format!("(match) {}", regex.as_str()),
        ExpectedValue::Array { items, unordered } => {
            let prefix = if *unordered { "(unordered) " } else { "" };
            format!("{prefix}array[{}]", items.len())
        }
    }
}

fn substitute_captures(
    command: &str,
    captures: &std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            let mut closed = false;

            for next in chars.by_ref() {
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }

            if !closed {
                return Err(format!(
                    "unterminated capture reference in command `{command}`"
                ));
            }

            let value = captures
                .get(&name)
                .ok_or_else(|| format!("unknown capture `{name}` in command `{command}`"))?;
            out.push_str(value);
            continue;
        }

        out.push(ch);
    }

    Ok(out)
}

fn render_expected_bytes(bytes: &[u8]) -> String {
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

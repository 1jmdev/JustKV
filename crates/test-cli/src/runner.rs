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

    let discovered_total = files.iter().map(|file| file.cases.len()).sum();

    let preflight = filter_cases_by_capability(&args, files).await;
    let (files, mut warnings, prefiltered_skipped) = match preflight {
        Ok(report) => (report.files, report.warnings, report.skipped),
        Err(error) => (
            paths
                .iter()
                .map(|path| parse_test_file(path))
                .collect::<Result<Vec<_>, _>>()?,
            vec![format!("failed to probe server capabilities; running without prefiltering: {error}")],
            0usize,
        ),
    };

    let total = files.iter().map(|file| file.cases.len()).sum();
    let ui = Ui::new(total, args.quiet);
    ui.set_discovery(files.len(), total);

    let mut failures = Vec::new();
    let mut passed = 0usize;
    let mut skipped = 0usize;

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
                CaseOutcome::Passed => {
                    passed += 1;
                    ui.record_success(&location);
                }
                CaseOutcome::Skipped => {
                    skipped += 1;
                    ui.record_success(&location);
                }
                CaseOutcome::Failed(error) => {
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
        discovered_total,
        total,
        passed,
        skipped: skipped + prefiltered_skipped,
        failed: failures.len(),
        elapsed: started.elapsed(),
        failures,
        warnings: {
            if prefiltered_skipped != 0 {
                warnings.push(format!(
                    "preflight skipped {prefiltered_skipped} test(s) due to unsupported capabilities or unsafe commands"
                ));
            }
            warnings
        },
    };

    ui.finish(&summary);
    print_failures(&summary.failures);

    Ok(summary)
}

struct PreflightReport {
    files: Vec<crate::model::TestFile>,
    skipped: usize,
    warnings: Vec<String>,
}

async fn filter_cases_by_capability(
    args: &Args,
    files: Vec<crate::model::TestFile>,
) -> Result<PreflightReport, String> {
    let mut client = Client::connect(args).await?;
    let mut warnings = Vec::new();

    let mut commands = std::collections::BTreeSet::new();
    for file in &files {
        for case in &file.cases {
            for command in case.setup.iter().chain(case.run.iter()).chain(case.cleanup.iter()) {
                if let Some(name) = command_name(command) {
                    commands.insert(name);
                }
            }
        }
    }

    let mut supported = std::collections::HashMap::new();
    let command_info_usable = match command_supported(&mut client, "PING").await {
        Ok(true) => true,
        Ok(false) => {
            warnings.push(
                "COMMAND INFO probing is unavailable on this server: `COMMAND INFO PING` returned an empty result, so command-support filtering was disabled"
                    .to_string(),
            );
            false
        }
        Err(error) => {
            warnings.push(format!(
                "could not probe COMMAND INFO support: {error}; command-support filtering was disabled"
            ));
            false
        }
    };

    for command in commands {
        if !command_info_usable {
            supported.insert(command, None);
            continue;
        }

        match command_supported(&mut client, &command).await {
            Ok(is_supported) => {
                supported.insert(command.clone(), Some(is_supported));
            }
            Err(error) => {
                warnings.push(format!(
                    "could not probe COMMAND INFO for `{command}`: {error}; tests using it will still run"
                ));
                supported.insert(command.clone(), None);
            }
        }
    }

    let cluster_enabled = probe_bool(
        &mut client,
        vec![b"CLUSTER".to_vec(), b"INFO".to_vec()],
        "ERR This instance has cluster support disabled",
        "CLUSTER support",
        &mut warnings,
    )
    .await;

    let acl_file_configured = probe_bool(
        &mut client,
        vec![b"ACL".to_vec(), b"SAVE".to_vec()],
        "ERR This Redis instance is not configured to use an ACL file",
        "ACL file support",
        &mut warnings,
    )
    .await;

    let config_rewrite_supported = probe_bool(
        &mut client,
        vec![b"CONFIG".to_vec(), b"REWRITE".to_vec()],
        "ERR The server is running without a config file",
        "CONFIG REWRITE support",
        &mut warnings,
    )
    .await;

    let object_freq_supported = match client
        .execute(vec![b"SET".to_vec(), b"__betterkv_tester_probe__".to_vec(), b"1".to_vec()])
        .await
    {
        Ok(_) => {
            probe_bool(
                &mut client,
                vec![b"OBJECT".to_vec(), b"FREQ".to_vec(), b"__betterkv_tester_probe__".to_vec()],
                "ERR An LFU maxmemory policy is not selected, access frequency not tracked.",
                "OBJECT FREQ support",
                &mut warnings,
            )
            .await
        }
        Err(error) => {
            warnings.push(format!(
                "could not initialize OBJECT FREQ probe: {error}; OBJECT FREQ tests will still run"
            ));
            None
        }
    };
    let _ = client.execute(vec![b"DEL".to_vec(), b"__betterkv_tester_probe__".to_vec()]).await;

    let mut filtered = Vec::with_capacity(files.len());
    let mut skipped = 0usize;
    for mut file in files {
        let before = file.cases.len();
        file.cases.retain(|case| {
            case_supported(
                case,
                &supported,
                cluster_enabled,
                acl_file_configured,
                config_rewrite_supported,
                object_freq_supported,
            )
        });
        skipped += before - file.cases.len();
        if !file.cases.is_empty() {
            filtered.push(file);
        }
    }

    Ok(PreflightReport {
        files: filtered,
        skipped,
        warnings,
    })
}

async fn command_supported(client: &mut Client, command: &str) -> Result<bool, String> {
    let frame = client
        .execute(vec![b"COMMAND".to_vec(), b"INFO".to_vec(), command.as_bytes().to_vec()])
        .await?;
    Ok(match frame {
        RespFrame::Array(Some(items)) if items.is_empty() => false,
        RespFrame::Array(Some(items)) if items.len() == 1 => !matches!(&items[0], RespFrame::Bulk(None) | RespFrame::Array(None)),
        RespFrame::BulkOptions(items) if items.len() == 1 => items[0].is_some(),
        _ => true,
    })
}

fn case_supported(
    case: &crate::model::TestCase,
    supported: &std::collections::HashMap<String, Option<bool>>,
    cluster_enabled: Option<bool>,
    acl_file_configured: Option<bool>,
    config_rewrite_supported: Option<bool>,
    object_freq_supported: Option<bool>,
) -> bool {
    for command in case.setup.iter().chain(case.run.iter()).chain(case.cleanup.iter()) {
        if let Some(name) = command_name(command) {
            if matches!(name.as_str(), "SHUTDOWN" | "REPLICAOF" | "SLAVEOF" | "MIGRATE") {
                return false;
            }
            if matches!(supported.get(&name), Some(Some(false))) {
                return false;
            }
        }

        let upper = command.trim().to_ascii_uppercase();
        if upper.starts_with("CLUSTER ") && matches!(cluster_enabled, Some(false)) {
            return false;
        }
        if matches!(upper.as_str(), "ASKING" | "READONLY" | "READWRITE")
            && matches!(cluster_enabled, Some(false))
        {
            return false;
        }
        if matches!(upper.as_str(), "ACL SAVE" | "ACL LOAD")
            && matches!(acl_file_configured, Some(false))
        {
            return false;
        }
        if upper == "CONFIG REWRITE" && matches!(config_rewrite_supported, Some(false)) {
            return false;
        }
        if upper.starts_with("OBJECT FREQ ") && matches!(object_freq_supported, Some(false)) {
            return false;
        }
    }

    true
}

async fn probe_bool(
    client: &mut Client,
    command: Vec<Vec<u8>>,
    unsupported_prefix: &str,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<bool> {
    match client.execute(command).await {
        Ok(frame) => Some(!frame_error_starts_with(&frame, unsupported_prefix)),
        Err(error) => {
            warnings.push(format!(
                "could not determine {label}: {error}; related tests will still run"
            ));
            None
        }
    }
}

fn command_name(command: &str) -> Option<String> {
    command
        .split_ascii_whitespace()
        .next()
        .map(|value| value.to_ascii_uppercase())
}

fn frame_error_starts_with(frame: &RespFrame, prefix: &str) -> bool {
    match frame {
        RespFrame::Error(value) => value.starts_with(prefix),
        RespFrame::ErrorStatic(value) => value.starts_with(prefix),
        _ => false,
    }
}

enum CaseOutcome {
    Passed,
    Skipped,
    Failed(String),
}

async fn run_case(args: &Args, case: &crate::model::TestCase) -> CaseOutcome {
    if case_skip_reason(case).is_some() {
        return CaseOutcome::Skipped;
    }

    let mut client = match Client::connect(args).await {
        Ok(client) => client,
        Err(error) => return CaseOutcome::Failed(error),
    };
    if let Err(error) = client.flush_all().await {
        return CaseOutcome::Failed(format!("FLUSHALL failed: {error}"));
    }
    let mut captures = std::collections::HashMap::<String, String>::new();

    let mut outcome = CaseOutcome::Passed;

    for command in &case.setup {
        let command = match substitute_captures(command, &captures) {
            Ok(command) => command,
            Err(error) => {
                outcome = CaseOutcome::Failed(error);
                break;
            }
        };
        let frame = match client.execute_raw(&command).await {
            Ok(frame) => frame,
            Err(error) => {
                outcome = CaseOutcome::Failed(format!("setup command `{command}` failed: {error}"));
                break;
            }
        };
        if response_skip_reason(&frame).is_some() {
            outcome = CaseOutcome::Skipped;
            break;
        }
    }

    if matches!(outcome, CaseOutcome::Passed) {
        let no_reply = matches!(case.expect, ExpectedValue::NoReply);
        let sequence_expectations = match &case.expect {
            ExpectedValue::Sequence(items) if items.len() == case.run.len() => Some(items.as_slice()),
            _ => None,
        };
        let mut responses = Vec::with_capacity(case.run.len());
        let last_index = case.run.len().saturating_sub(1);
        for (index, command) in case.run.iter().enumerate() {
            let command = match substitute_captures(command, &captures) {
                Ok(command) => command,
                Err(error) => {
                    outcome = CaseOutcome::Failed(error);
                    break;
                }
            };
            if no_reply && index == last_index {
                if let Err(error) = client.execute_raw_no_reply(&command).await {
                    outcome = CaseOutcome::Failed(format!("run command `{command}` failed: {error}"));
                    break;
                }
            } else {
                let frame = match client.execute_raw(&command).await {
                    Ok(frame) => frame,
                    Err(error) => {
                        outcome = CaseOutcome::Failed(format!("run command `{command}` failed: {error}"));
                        break;
                    }
                };
                if response_skip_reason(&frame).is_some() {
                    outcome = CaseOutcome::Skipped;
                    break;
                }
                responses.push(frame);
                if let Some(ExpectedValue::Capture(_)) = sequence_expectations.and_then(|items| items.get(index)) {
                    if let Err(error) = validate_expected(
                        sequence_expectations.unwrap().get(index).unwrap(),
                        responses.last().unwrap(),
                        &mut captures,
                    ) {
                        outcome = CaseOutcome::Failed(error);
                        break;
                    }
                }
            }
        }

        if matches!(outcome, CaseOutcome::Passed) && !no_reply {
            if let Err(error) = validate_run_results(&case.expect, &responses, &mut captures) {
                outcome = CaseOutcome::Failed(error);
            }
        }
    }

    for command in &case.cleanup {
        let command = match substitute_captures(command, &captures) {
            Ok(command) => command,
            Err(error) => {
                if matches!(outcome, CaseOutcome::Passed) {
                    outcome = CaseOutcome::Failed(error);
                }
                break;
            }
        };
        let frame = match client.execute_raw(&command).await {
            Ok(frame) => frame,
            Err(error) => {
                if matches!(outcome, CaseOutcome::Passed) {
                    outcome = CaseOutcome::Failed(format!("cleanup command `{command}` failed: {error}"));
                }
                break;
            }
        };
        if response_skip_reason(&frame).is_some() {
            if matches!(outcome, CaseOutcome::Passed) {
                outcome = CaseOutcome::Skipped;
            }
            break;
        }
    }

    outcome
}

fn case_skip_reason(case: &crate::model::TestCase) -> Option<String> {
    for command in case.setup.iter().chain(case.run.iter()).chain(case.cleanup.iter()) {
        let name = command
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if matches!(name.as_str(), "SHUTDOWN" | "REPLICAOF" | "SLAVEOF" | "MIGRATE") {
            return Some(format!("skipped unsafe command `{name}`"));
        }
    }
    None
}

fn response_skip_reason(frame: &RespFrame) -> Option<String> {
    let message = match frame {
        RespFrame::Error(value) => value.as_str(),
        RespFrame::ErrorStatic(value) => value,
        _ => return None,
    };

    let unsupported = [
        "ERR unknown command",
        "ERR This instance has cluster support disabled",
        "ERR The server is running without a config file",
        "ERR This Redis instance is not configured to use an ACL file",
        "ERR An LFU maxmemory policy is not selected, access frequency not tracked.",
    ];

    unsupported
        .iter()
        .find(|prefix| message.starts_with(**prefix))
        .map(|_| format!("skipped unsupported server capability: {message}"))
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
            RespFrame::Bulk(None) | RespFrame::Array(None) => Ok(()),
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
            let raw = render_frame_raw(actual);
            let rendered = render_frame(actual);
            if regex_matches(regex, &raw, &rendered) {
                Ok(())
            } else {
                Err(format!(
                    "expected response to match `{}`, got:\n{}",
                    regex.as_str(),
                    rendered
                ))
            }
        }
        ExpectedValue::Array { items, unordered } => {
            validate_array(items, *unordered, actual, captures)
        }
    }
}

fn regex_matches(regex: &regex::Regex, raw: &str, rendered: &str) -> bool {
    if regex.is_match(raw) || regex.is_match(rendered) {
        return true;
    }

    let escaped = escape_unescaped_parens(regex.as_str());
    if escaped == regex.as_str() {
        return false;
    }

    match regex::Regex::new(&escaped) {
        Ok(fallback) => fallback.is_match(raw) || fallback.is_match(rendered),
        Err(_) => false,
    }
}

fn escape_unescaped_parens(pattern: &str) -> String {
    let mut escaped = String::with_capacity(pattern.len());
    let mut backslashes = 0usize;

    for ch in pattern.chars() {
        let is_escaped = backslashes % 2 == 1;
        if matches!(ch, '(' | ')') && !is_escaped {
            escaped.push('\\');
        }
        escaped.push(ch);
        if ch == '\\' {
            backslashes += 1;
        } else {
            backslashes = 0;
        }
    }

    escaped
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

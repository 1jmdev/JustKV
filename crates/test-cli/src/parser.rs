use std::fs;
use std::path::Path;

use regex::Regex;

use crate::model::{ExpectedValue, FileMetadata, TestCase, TestFile};
use crate::syntax::parse_quoted_bytes;

pub fn parse_test_file(path: &Path) -> Result<TestFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;

    let lines = raw.lines().collect::<Vec<_>>();
    let mut metadata = FileMetadata::default();
    let mut cases = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim();
        if line.starts_with("@name ") {
            metadata.name = Some(line[6..].trim().to_string());
            index += 1;
            continue;
        }
        if line.starts_with("@group ") {
            metadata.group = Some(line[7..].trim().to_string());
            index += 1;
            continue;
        }
        if line.starts_with("@since ") {
            metadata.since = Some(line[7..].trim().to_string());
            index += 1;
            continue;
        }
        if line.starts_with("===") {
            let (case, next) = parse_case(&lines, index, path)?;
            cases.push(case);
            index = next;
            continue;
        }
        index += 1;
    }

    if cases.is_empty() {
        return Err(format!("{} does not contain any tests", path.display()));
    }

    Ok(TestFile {
        path: path.to_path_buf(),
        metadata,
        cases,
    })
}

fn parse_case(lines: &[&str], start: usize, path: &Path) -> Result<(TestCase, usize), String> {
    let header = lines[start].trim();
    let mut index = start + 1;

    let name = if let Some(name) = header.strip_prefix("=== TEST:") {
        name.trim().to_string()
    } else {
        while index < lines.len() && ignored_line(lines[index]) {
            index += 1;
        }
        let Some(name_line) = lines.get(index).map(|value| value.trim()) else {
            return Err(format!(
                "{}:{} missing test name",
                path.display(),
                start + 1
            ));
        };
        let Some(name) = name_line.strip_prefix("--- TEST:") else {
            return Err(format!(
                "{}:{} expected `--- TEST:` after test separator",
                path.display(),
                start + 1
            ));
        };
        index += 1;
        name.trim().to_string()
    };

    let mut setup = Vec::new();
    let mut run = Vec::new();
    let mut cleanup = Vec::new();
    let mut expect_lines = Vec::new();
    let mut section = Section::None;

    while index < lines.len() {
        let raw_line = lines[index];
        let line = raw_line.trim();
        if index != start && line.starts_with("===") {
            break;
        }
        if line == "---" {
            break;
        }
        if ignored_line(raw_line) {
            index += 1;
            continue;
        }

        match line {
            "SETUP:" => section = Section::Setup,
            "RUN:" => section = Section::Run,
            "EXPECT:" => section = Section::Expect,
            "CLEANUP:" => section = Section::Cleanup,
            _ => match section {
                Section::Setup => setup.push(line.to_string()),
                Section::Run => run.push(line.to_string()),
                Section::Expect => expect_lines.push(raw_line.trim_end().to_string()),
                Section::Cleanup => cleanup.push(line.to_string()),
                Section::None => {
                    return Err(format!(
                        "{}:{} unexpected content outside a section",
                        path.display(),
                        index + 1
                    ));
                }
            },
        }

        index += 1;
    }

    if run.is_empty() {
        return Err(format!(
            "{}:{} test `{name}` is missing RUN section",
            path.display(),
            start + 1
        ));
    }
    if expect_lines.is_empty() {
        return Err(format!(
            "{}:{} test `{name}` is missing EXPECT section",
            path.display(),
            start + 1
        ));
    }

    let expect = parse_expected(&expect_lines, path, &name)?;
    Ok((
        TestCase {
            name,
            setup,
            run,
            expect,
            cleanup,
        },
        index,
    ))
}

fn parse_expected(lines: &[String], path: &Path, test_name: &str) -> Result<ExpectedValue, String> {
    let mut groups = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim();

        if line == "(unordered)" {
            let next = lines.get(index + 1).ok_or_else(|| {
                format!(
                    "{} test `{test_name}` has `(unordered)` with no following array",
                    path.display()
                )
            })?;
            if !is_numbered_line(next) {
                return Err(format!(
                    "{} test `{test_name}` uses `(unordered)` without an array expectation",
                    path.display()
                ));
            }

            let (array, next_index) = parse_array_group(lines, index + 1, true, path, test_name)?;
            groups.push(array);
            index = next_index;
            continue;
        }

        if is_numbered_line(&lines[index]) {
            let (array, next_index) = parse_array_group(lines, index, false, path, test_name)?;
            groups.push(array);
            index = next_index;
            continue;
        }

        groups.push(parse_scalar_expected(line, false, path, test_name)?);
        index += 1;
    }

    if groups.len() == 1 {
        return Ok(groups.remove(0));
    }

    Ok(ExpectedValue::Sequence(groups))
}

fn parse_array_group(
    lines: &[String],
    start: usize,
    unordered: bool,
    path: &Path,
    test_name: &str,
) -> Result<(ExpectedValue, usize), String> {
    let base_indent = line_indent(&lines[start]);
    let mut items = Vec::new();
    let mut index = start;

    while index < lines.len() {
        if !is_numbered_line_with_indent(&lines[index], base_indent) {
            break;
        }

        let (_, content) = split_numbered_line(lines[index].trim()).ok_or_else(|| {
            format!(
                "{} test `{test_name}` has invalid array line `{}`",
                path.display(),
                lines[index].trim()
            )
        })?;
        index += 1;

        if is_inline_numbered(content) {
            let mut nested_lines = vec![content.to_string()];
            while index < lines.len() && line_indent(&lines[index]) > base_indent {
                nested_lines.push(lines[index].trim().to_string());
                index += 1;
            }
            let (nested, consumed) = parse_array_group(&nested_lines, 0, false, path, test_name)?;
            debug_assert_eq!(consumed, nested_lines.len());
            items.push(nested);
            continue;
        }

        items.push(parse_scalar_expected(content, true, path, test_name)?);

        while index < lines.len() && line_indent(&lines[index]) > base_indent {
            return Err(format!(
                "{} test `{test_name}` has unexpected indented array line `{}`",
                path.display(),
                lines[index].trim()
            ));
        }
    }

    Ok((ExpectedValue::Array { items, unordered }, index))
}

fn is_numbered_line(line: &str) -> bool {
    split_numbered_line(line.trim()).is_some()
}

fn is_numbered_line_with_indent(line: &str, indent: usize) -> bool {
    line_indent(line) == indent && is_numbered_line(line)
}

fn is_inline_numbered(line: &str) -> bool {
    split_numbered_line(line.trim()).is_some()
}

fn split_numbered_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let (prefix, value) = trimmed.split_once(')')?;
    if prefix.is_empty() || !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((prefix, value.trim()))
}

fn line_indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn parse_scalar_expected(
    raw: &str,
    in_array: bool,
    path: &Path,
    test_name: &str,
) -> Result<ExpectedValue, String> {
    if raw == "(any)" {
        return Ok(ExpectedValue::Any);
    }
    if let Some(rest) = raw.strip_prefix("(capture) ") {
        return Ok(ExpectedValue::Capture(rest.trim().to_string()));
    }
    if raw == "(noreply)" || raw == "(no reply)" || raw == "(no response)" {
        return Ok(ExpectedValue::NoReply);
    }
    if raw == "(nil)" {
        return Ok(ExpectedValue::Bulk(None));
    }
    if raw == "(empty array)" || raw == "(empty list or set)" {
        return Ok(ExpectedValue::EmptyArray);
    }
    if raw == "(error)" {
        return Ok(ExpectedValue::ErrorAny);
    }
    if let Some(rest) = raw.strip_prefix("(error) ") {
        return Ok(ExpectedValue::ErrorPrefix(rest.trim().to_string()));
    }
    if let Some(rest) = raw.strip_prefix("(integer) ") {
        if rest.trim() == "(any)" {
            return Ok(ExpectedValue::IntegerAny);
        }
        let value = rest.trim().parse::<i64>().map_err(|err| {
            format!(
                "{} test `{test_name}` has invalid integer expectation `{raw}`: {err}",
                path.display()
            )
        })?;
        return Ok(ExpectedValue::Integer(value));
    }
    if let Some(rest) = raw.strip_prefix("(match) ") {
        let pattern = rest.trim().replace("\\\\", "\\");
        let regex = Regex::new(&pattern).map_err(|err| {
            format!(
                "{} test `{test_name}` has invalid regex `{}`: {err}",
                path.display(),
                pattern
            )
        })?;
        return Ok(ExpectedValue::RawRegex(regex));
    }
    if raw.starts_with('"') {
        return Ok(ExpectedValue::Bulk(Some(parse_quoted_bytes(raw).map_err(
            |err| format!("{} test `{test_name}`: {err}", path.display()),
        )?)));
    }
    if !in_array {
        return Ok(ExpectedValue::Simple(raw.to_string()));
    }

    Ok(ExpectedValue::Simple(raw.to_string()))
}
fn ignored_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

#[derive(Debug, Clone, Copy)]
enum Section {
    None,
    Setup,
    Run,
    Expect,
    Cleanup,
}

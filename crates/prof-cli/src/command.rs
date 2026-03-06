use protocol::types::RespFrame;

pub fn parse_command(input: &str) -> Result<Vec<Vec<u8>>, String> {
    let parts = shlex_split(input)?;
    if parts.is_empty() {
        return Err("empty command".to_string());
    }
    Ok(parts.into_iter().map(String::into_bytes).collect())
}

fn shlex_split(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
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

pub fn format_resp(frame: &RespFrame) -> String {
    match frame {
        RespFrame::Simple(value) => value.clone(),
        RespFrame::SimpleStatic(value) => (*value).to_string(),
        RespFrame::Error(value) => format!("(error) {value}"),
        RespFrame::ErrorStatic(value) => format!("(error) {value}"),
        RespFrame::Integer(value) => format!("(integer) {value}"),
        RespFrame::Bulk(None) => "(nil)".to_string(),
        RespFrame::Bulk(Some(value)) => String::from_utf8_lossy(value.as_slice()).into_owned(),
        RespFrame::Array(None) => "(empty)".to_string(),
        RespFrame::Array(Some(items)) => items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}) {}", i + 1, format_resp(item)))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "(other)".to_string(),
    }
}

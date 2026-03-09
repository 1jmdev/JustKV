pub fn parse_command_line(input: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut parts = Vec::new();
    let bytes = input.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if index >= bytes.len() {
            break;
        }

        let (part, next) = if bytes[index] == b'"' {
            parse_quoted_bytes_with_offset(input, index)?
        } else if bytes[index] == b'\'' {
            parse_single_quoted_bytes_with_offset(input, index)?
        } else {
            parse_unquoted_bytes(input, index)?
        };

        parts.push(part);
        index = next;
    }

    if parts.is_empty() {
        return Err("empty command".to_string());
    }

    Ok(parts)
}

pub fn parse_quoted_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let (value, next) = parse_quoted_bytes_with_offset(raw, 0)?;
    if next != raw.len() {
        return Err(format!("invalid trailing content in quoted string `{raw}`"));
    }
    Ok(value)
}

fn parse_quoted_bytes_with_offset(raw: &str, start: usize) -> Result<(Vec<u8>, usize), String> {
    let bytes = raw.as_bytes();
    if bytes.get(start) != Some(&b'"') {
        return Err(format!("invalid quoted string `{raw}`"));
    }

    let mut out = Vec::new();
    let mut index = start + 1;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Ok((out, index + 1)),
            b'\\' => {
                index += 1;
                let escaped = *bytes
                    .get(index)
                    .ok_or_else(|| format!("invalid escape in `{raw}`"))?;
                match escaped {
                    b'\\' => out.push(b'\\'),
                    b'"' => out.push(b'"'),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'0' => out.push(0),
                    b'x' => {
                        let hi = *bytes
                            .get(index + 1)
                            .ok_or_else(|| format!("invalid hex escape in `{raw}`"))?;
                        let lo = *bytes
                            .get(index + 2)
                            .ok_or_else(|| format!("invalid hex escape in `{raw}`"))?;
                        out.push((decode_hex(hi, raw)? << 4) | decode_hex(lo, raw)?);
                        index += 2;
                    }
                    other => {
                        return Err(format!(
                            "unsupported escape `\\{}` in `{raw}`",
                            other as char
                        ));
                    }
                }
            }
            byte => out.push(byte),
        }

        index += 1;
    }

    Err(format!("unterminated quoted string `{raw}`"))
}

fn parse_single_quoted_bytes_with_offset(
    raw: &str,
    start: usize,
) -> Result<(Vec<u8>, usize), String> {
    let bytes = raw.as_bytes();
    if bytes.get(start) != Some(&b'\'') {
        return Err(format!("invalid single-quoted string `{raw}`"));
    }

    let mut out = Vec::new();
    let mut index = start + 1;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' => return Ok((out, index + 1)),
            byte => out.push(byte),
        }

        index += 1;
    }

    Err(format!("unterminated single-quoted string `{raw}`"))
}

fn parse_unquoted_bytes(raw: &str, start: usize) -> Result<(Vec<u8>, usize), String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::new();
    let mut index = start;

    while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
        if bytes[index] == b'\\' {
            index += 1;
            let escaped = *bytes
                .get(index)
                .ok_or_else(|| format!("invalid escape in `{raw}`"))?;
            match escaped {
                b' ' => out.push(b' '),
                b'\\' => out.push(b'\\'),
                b'"' => out.push(b'"'),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'0' => out.push(0),
                b'x' => {
                    let hi = *bytes
                        .get(index + 1)
                        .ok_or_else(|| format!("invalid hex escape in `{raw}`"))?;
                    let lo = *bytes
                        .get(index + 2)
                        .ok_or_else(|| format!("invalid hex escape in `{raw}`"))?;
                    out.push((decode_hex(hi, raw)? << 4) | decode_hex(lo, raw)?);
                    index += 2;
                }
                other => {
                    return Err(format!(
                        "unsupported escape `\\{}` in `{raw}`",
                        other as char
                    ));
                }
            }
        } else {
            out.push(bytes[index]);
        }

        index += 1;
    }

    Ok((out, index))
}

fn decode_hex(value: u8, raw: &str) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(format!("invalid hex escape in `{raw}`")),
    }
}

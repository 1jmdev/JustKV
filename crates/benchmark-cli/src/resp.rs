use bytes::{Buf, BytesMut};
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Clone, Debug)]
pub enum ExpectedResponse {
    Simple(&'static str),
    Bulk(Option<Vec<u8>>),
    Integer(i64),
    IntegerRange { min: i64, max: i64 },
    Array(Vec<ExpectedResponse>),
}

pub fn encode_resp_parts(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|part| part.len() + 16).sum::<usize>() + 16);
    append_resp_parts(&mut out, parts);
    out
}

pub fn repeat_payload(one: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(one.len() * count);
    for _ in 0..count {
        out.extend_from_slice(one);
    }
    out
}

pub fn append_resp_parts(out: &mut Vec<u8>, parts: &[&[u8]]) {
    out.push(b'*');
    append_u64(out, parts.len() as u64);
    out.extend_from_slice(b"\r\n");

    for part in parts {
        out.push(b'$');
        append_u64(out, part.len() as u64);
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
}

pub fn make_key_into(base: &[u8], sequence: u64, key: &mut Vec<u8>) {
    key.clear();
    key.extend_from_slice(base);
    if sequence != 0 {
        key.push(b':');
        append_u64(key, sequence);
    }
}

pub async fn read_n_responses(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: usize,
) -> Result<(), String> {
    for _ in 0..expected {
        let frame = read_one_response(stream, parse_buf).await?;
        if let RespFrame::Error(message) = frame {
            return Err(format!("server returned error: {message}"));
        }
        if let RespFrame::ErrorStatic(message) = frame {
            return Err(format!("server returned error: {message}"));
        }
    }
    Ok(())
}

pub async fn read_n_strict_responses(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: &[ExpectedResponse],
    encoded: &[Option<Vec<u8>>],
    mut on_response: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for (expected_response, encoded_response) in expected.iter().zip(encoded.iter()) {
        if let Some(encoded_response) = encoded_response {
            validate_exact_response(stream, parse_buf, encoded_response).await?;
        } else {
            let frame = read_one_response(stream, parse_buf).await?;
            validate_response(expected_response, &frame)?;
        }
        on_response()?;
    }

    Ok(())
}

pub async fn read_n_strict_repeated_exact_responses(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: &[u8],
    count: usize,
    mut on_response: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..count {
        validate_exact_response(stream, parse_buf, expected).await?;
        on_response()?;
    }

    Ok(())
}

pub async fn read_n_unchecked_responses(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    encoded: &[Option<Vec<u8>>],
    mut on_response: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for encoded_response in encoded {
        if let Some(encoded) = encoded_response {
            skip_exact_response(stream, parse_buf, encoded).await?;
        } else {
            skip_one_response(stream, parse_buf).await?;
        }
        on_response()?;
    }

    Ok(())
}

pub async fn read_n_unchecked_repeated_exact_responses(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: &[u8],
    count: usize,
    mut on_response: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..count {
        skip_exact_response(stream, parse_buf, expected).await?;
        on_response()?;
    }

    Ok(())
}

pub async fn read_one_response(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
) -> Result<RespFrame, String> {
    loop {
        match parser::parse_frame(parse_buf) {
            Ok(Some(frame)) => return Ok(frame),
            Ok(None) | Err(ParseError::Incomplete) => {}
            Err(ParseError::Protocol(err)) => return Err(format!("protocol error: {err}")),
        }

        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }
}

async fn skip_exact_response(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: &[u8],
) -> Result<(), String> {
    while parse_buf.len() < expected.len() {
        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }

    if parse_buf.first().copied() == Some(b'-') {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("server returned error while skipping: {frame:?}"));
    }

    parse_buf.advance(expected.len());
    Ok(())
}

async fn validate_exact_response(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
    expected: &[u8],
) -> Result<(), String> {
    while parse_buf.len() < expected.len() {
        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }

    if parse_buf.first().copied() == Some(b'-') {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("server returned error while validating: {frame:?}"));
    }

    if &parse_buf[..expected.len()] != expected {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("unexpected response bytes, got {frame:?}"));
    }

    parse_buf.advance(expected.len());
    Ok(())
}

async fn skip_one_response(
    stream: &mut (impl AsyncRead + Unpin),
    parse_buf: &mut BytesMut,
) -> Result<(), String> {
    loop {
        match try_skip_frame(parse_buf)? {
            Some(()) => return Ok(()),
            None => {
                let read = stream
                    .read_buf(parse_buf)
                    .await
                    .map_err(|err| format!("read failed: {err}"))?;
                if read == 0 {
                    return Err("connection closed by server".to_string());
                }
            }
        }
    }
}

pub fn encode_expected_response(expected: &ExpectedResponse) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    append_expected_response(&mut out, expected)?;
    Some(out)
}

fn append_expected_response(out: &mut Vec<u8>, expected: &ExpectedResponse) -> Option<()> {
    match expected {
        ExpectedResponse::Simple(value) => {
            out.push(b'+');
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        ExpectedResponse::Bulk(None) => out.extend_from_slice(b"$-1\r\n"),
        ExpectedResponse::Bulk(Some(value)) => {
            out.push(b'$');
            append_u64(out, value.len() as u64);
            out.extend_from_slice(b"\r\n");
            out.extend_from_slice(value);
            out.extend_from_slice(b"\r\n");
        }
        ExpectedResponse::Integer(value) => {
            out.push(b':');
            append_i64(out, *value);
            out.extend_from_slice(b"\r\n");
        }
        ExpectedResponse::IntegerRange { .. } => return None,
        ExpectedResponse::Array(items) => {
            out.push(b'*');
            append_u64(out, items.len() as u64);
            out.extend_from_slice(b"\r\n");
            for item in items {
                append_expected_response(out, item)?;
            }
        }
    }
    Some(())
}

fn validate_response(expected: &ExpectedResponse, actual: &RespFrame) -> Result<(), String> {
    match (expected, actual) {
        (ExpectedResponse::Simple(expected), RespFrame::Simple(actual)) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected simple {expected:?}, got {actual:?}"))
            }
        }
        (ExpectedResponse::Simple(expected), RespFrame::SimpleStatic(actual)) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected simple {expected:?}, got {actual:?}"))
            }
        }
        (ExpectedResponse::Bulk(None), RespFrame::Bulk(None)) => Ok(()),
        (ExpectedResponse::Bulk(Some(expected)), RespFrame::Bulk(Some(BulkData::Arg(actual)))) => {
            validate_bytes(expected, actual.as_slice())
        }
        (
            ExpectedResponse::Bulk(Some(expected)),
            RespFrame::Bulk(Some(BulkData::Value(actual))),
        ) => validate_bytes(expected, actual.as_slice()),
        (ExpectedResponse::Integer(expected), RespFrame::Integer(actual)) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected integer {expected}, got {actual}"))
            }
        }
        (ExpectedResponse::IntegerRange { min, max }, RespFrame::Integer(actual)) => {
            if (*min..=*max).contains(actual) {
                Ok(())
            } else {
                Err(format!("expected integer in [{min}, {max}], got {actual}"))
            }
        }
        (ExpectedResponse::Array(expected_items), RespFrame::Array(Some(actual_items))) => {
            if expected_items.len() != actual_items.len() {
                return Err(format!(
                    "expected array len {}, got {}",
                    expected_items.len(),
                    actual_items.len()
                ));
            }
            for (expected_item, actual_item) in expected_items.iter().zip(actual_items.iter()) {
                validate_response(expected_item, actual_item)?;
            }
            Ok(())
        }
        _ => Err(format!(
            "unexpected response shape: expected {expected:?}, got {actual:?}"
        )),
    }
}

fn validate_bytes(expected: &[u8], actual: &[u8]) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected bulk {:?}, got {:?}",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(actual)
        ))
    }
}

fn append_i64(out: &mut Vec<u8>, value: i64) {
    let mut tmp = itoa::Buffer::new();
    out.extend_from_slice(tmp.format(value).as_bytes());
}

fn try_skip_frame(buf: &mut BytesMut) -> Result<Option<()>, String> {
    let Some(consumed) = frame_len(buf.as_ref(), 0)? else {
        return Ok(None);
    };
    buf.advance(consumed);
    Ok(Some(()))
}

fn frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    if start >= src.len() {
        return Ok(None);
    }

    match src[start] {
        b'+' | b'-' | b':' => line_frame_len(src, start),
        b'$' => bulk_frame_len(src, start),
        b'*' => aggregate_frame_len(src, start, 1),
        b'%' => aggregate_frame_len(src, start, 2),
        other => Err(format!("unsupported RESP type byte: {other:?}")),
    }
}

fn line_frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    Ok(Some(end + 2 - start))
}

fn bulk_frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    let len = parse_i64_ascii(&src[start + 1..end])?;
    if len < 0 {
        return Ok(Some(end + 2 - start));
    }
    let total = end + 2 + len as usize + 2;
    if src.len() < total {
        return Ok(None);
    }
    Ok(Some(total - start))
}

fn aggregate_frame_len(
    src: &[u8],
    start: usize,
    multiplier: usize,
) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    let count = parse_i64_ascii(&src[start + 1..end])?;
    if count < 0 {
        return Ok(Some(end + 2 - start));
    }

    let mut cursor = end + 2;
    for _ in 0..(count as usize * multiplier) {
        let Some(len) = frame_len(src, cursor)? else {
            return Ok(None);
        };
        cursor += len;
    }
    Ok(Some(cursor - start))
}

fn find_crlf(src: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    while index + 1 < src.len() {
        if src[index] == b'\r' && src[index + 1] == b'\n' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_i64_ascii(raw: &[u8]) -> Result<i64, String> {
    let text = std::str::from_utf8(raw).map_err(|err| format!("invalid integer bytes: {err}"))?;
    text.parse::<i64>()
        .map_err(|err| format!("invalid integer {text:?}: {err}"))
}

fn append_u64(out: &mut Vec<u8>, value: u64) {
    let mut tmp = itoa::Buffer::new();
    out.extend_from_slice(tmp.format(value).as_bytes());
}

use bytes::{Buf, BytesMut};
use thiserror::Error;

use crate::types::{BulkData, RespFrame};
use engine::value::CompactArg;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("incomplete frame")]
    Incomplete,
    #[error("protocol error: {0}")]
    Protocol(String),
}

pub fn parse_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_frame");
    if src.is_empty() {
        return Ok(None);
    }

    match parse_value(src, 0) {
        Ok((frame, consumed)) => {
            src.advance(consumed);
            Ok(Some(frame))
        }
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

fn parse_value(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_value");
    if offset >= src.len() {
        return Err(ParseError::Incomplete);
    }

    match src[offset] {
        b'+' => parse_simple(src, offset),
        b'-' => parse_error(src, offset),
        b':' => parse_integer(src, offset),
        b'$' => parse_bulk(src, offset),
        b'*' => parse_array(src, offset),
        _ => parse_inline(src, offset),
    }
}

fn parse_inline(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_inline");
    let (line, consumed) = parse_line_bytes(src, offset)?;
    let parts: Vec<RespFrame> = line
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(part)))))
        .collect();

    if parts.is_empty() {
        return Err(ParseError::Protocol("empty inline command".to_string()));
    }

    Ok((RespFrame::Array(Some(parts)), consumed))
}

fn parse_simple(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_simple");
    let (line, consumed) = parse_line_bytes(src, offset + 1)?;
    let text = std::str::from_utf8(line)
        .map_err(|_| ParseError::Protocol("invalid utf8 line".to_string()))?
        .to_owned();
    Ok((RespFrame::Simple(text), consumed))
}

fn parse_error(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_error");
    let (line, consumed) = parse_line_bytes(src, offset + 1)?;
    let text = std::str::from_utf8(line)
        .map_err(|_| ParseError::Protocol("invalid utf8 line".to_string()))?
        .to_owned();
    Ok((RespFrame::Error(text), consumed))
}

fn parse_integer(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_integer");
    let (line, consumed) = parse_line_bytes(src, offset + 1)?;
    let value = parse_decimal(line).ok_or(ParseError::Protocol("invalid integer".to_string()))?;
    Ok((RespFrame::Integer(value), consumed))
}

fn parse_bulk(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_bulk");
    let (line, mut cursor) = parse_line_bytes(src, offset + 1)?;
    let length =
        parse_decimal(line).ok_or(ParseError::Protocol("invalid bulk length".to_string()))?;

    if length < 0 {
        return Ok((RespFrame::Bulk(None), cursor));
    }

    let size = length as usize;
    if src.len() < cursor + size + 2 {
        return Err(ParseError::Incomplete);
    }

    let end = cursor + size;
    let payload = BulkData::Arg(CompactArg::from_slice(&src[cursor..end]));
    cursor = end;

    if src.get(cursor) != Some(&b'\r') || src.get(cursor + 1) != Some(&b'\n') {
        return Err(ParseError::Protocol("missing bulk terminator".to_string()));
    }

    cursor += 2;
    Ok((RespFrame::Bulk(Some(payload)), cursor))
}

fn parse_array(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_array");
    let (line, mut cursor) = parse_line_bytes(src, offset + 1)?;
    let length =
        parse_decimal(line).ok_or(ParseError::Protocol("invalid array length".to_string()))?;

    if length < 0 {
        return Ok((RespFrame::Array(None), cursor));
    }

    let mut items = Vec::with_capacity(length as usize);
    for _ in 0..length {
        let (item, consumed) = parse_value(src, cursor)?;
        cursor = consumed;
        items.push(item);
    }

    Ok((RespFrame::Array(Some(items)), cursor))
}

fn parse_line_bytes(src: &[u8], from: usize) -> Result<(&[u8], usize), ParseError> {
    let _trace = profiler::scope("protocol::parser::parse_line_bytes");
    let end = find_crlf(src, from).ok_or(ParseError::Incomplete)?;
    Ok((&src[from..end], end + 2))
}

fn find_crlf(src: &[u8], from: usize) -> Option<usize> {
    let _trace = profiler::scope("protocol::parser::find_crlf");
    let mut index = from;
    while index + 1 < src.len() {
        if src[index] == b'\r' && src[index + 1] == b'\n' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_decimal(raw: &[u8]) -> Option<i64> {
    let _trace = profiler::scope("protocol::parser::parse_decimal");
    if raw.is_empty() {
        return None;
    }

    let mut index = 0;
    let mut negative = false;
    if raw[0] == b'-' {
        negative = true;
        index = 1;
    }
    if index >= raw.len() {
        return None;
    }

    let mut value: i64 = 0;
    while index < raw.len() {
        let digit = raw[index].wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?;
        value = value.checked_add(i64::from(digit))?;
        index += 1;
    }

    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}

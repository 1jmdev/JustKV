use bytes::BytesMut;
use thiserror::Error;

use crate::protocol::types::RespFrame;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("incomplete frame")]
    Incomplete,
    #[error("protocol error: {0}")]
    Protocol(String),
}

pub fn parse_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
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
    let (line, consumed) = parse_line(src, offset)?;
    let parts: Vec<RespFrame> = line
        .split_ascii_whitespace()
        .map(|part| RespFrame::Bulk(Some(part.as_bytes().to_vec())))
        .collect();

    if parts.is_empty() {
        return Err(ParseError::Protocol("empty inline command".to_string()));
    }

    Ok((RespFrame::Array(Some(parts)), consumed))
}

fn parse_simple(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let (value, consumed) = parse_line(src, offset + 1)?;
    Ok((RespFrame::Simple(value), consumed))
}

fn parse_error(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let (value, consumed) = parse_line(src, offset + 1)?;
    Ok((RespFrame::Error(value), consumed))
}

fn parse_integer(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let (value, consumed) = parse_line(src, offset + 1)?;
    let parsed = value
        .parse::<i64>()
        .map_err(|_| ParseError::Protocol("invalid integer".to_string()))?;
    Ok((RespFrame::Integer(parsed), consumed))
}

fn parse_bulk(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let (length, mut cursor) = parse_line(src, offset + 1)?;
    let size = length
        .parse::<isize>()
        .map_err(|_| ParseError::Protocol("invalid bulk length".to_string()))?;

    if size < 0 {
        return Ok((RespFrame::Bulk(None), cursor));
    }

    let size = size as usize;
    if src.len() < cursor + size + 2 {
        return Err(ParseError::Incomplete);
    }

    let end = cursor + size;
    let payload = src[cursor..end].to_vec();
    cursor = end;

    if src.get(cursor) != Some(&b'\r') || src.get(cursor + 1) != Some(&b'\n') {
        return Err(ParseError::Protocol("missing bulk terminator".to_string()));
    }

    cursor += 2;
    Ok((RespFrame::Bulk(Some(payload)), cursor))
}

fn parse_array(src: &[u8], offset: usize) -> Result<(RespFrame, usize), ParseError> {
    let (length, mut cursor) = parse_line(src, offset + 1)?;
    let count = length
        .parse::<isize>()
        .map_err(|_| ParseError::Protocol("invalid array length".to_string()))?;

    if count < 0 {
        return Ok((RespFrame::Array(None), cursor));
    }

    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (item, consumed) = parse_value(src, cursor)?;
        cursor = consumed;
        items.push(item);
    }

    Ok((RespFrame::Array(Some(items)), cursor))
}

fn parse_line(src: &[u8], from: usize) -> Result<(String, usize), ParseError> {
    let end = find_crlf(src, from).ok_or(ParseError::Incomplete)?;
    let raw = &src[from..end];
    let value = std::str::from_utf8(raw)
        .map_err(|_| ParseError::Protocol("invalid utf8 line".to_string()))?
        .to_string();
    Ok((value, end + 2))
}

fn find_crlf(src: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < src.len() {
        if src[i] == b'\r' && src[i + 1] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}

use bytes::Buf;

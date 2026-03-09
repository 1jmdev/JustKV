use bytes::BytesMut;
use std::fmt;

use crate::types::{BulkData, RespFrame};
use types::value::{CompactArg, CompactValue};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Incomplete,
    Protocol(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => f.write_str("incomplete RESP frame"),
            Self::Protocol(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    #[inline(always)]
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[inline(always)]
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    #[inline(always)]
    fn peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(unsafe { *self.data.get_unchecked(self.pos) })
        } else {
            None
        }
    }

    /// Advance by `n` bytes.  Caller must guarantee `n <= remaining()`.
    #[inline(always)]
    unsafe fn advance_unchecked(&mut self, n: usize) {
        self.pos += n;
    }

    /// Return a slice `[pos .. pos+n)` and advance.
    #[inline(always)]
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.remaining() < n {
            return Err(ParseError::Incomplete);
        }
        let start = self.pos;
        self.pos += n;
        Ok(unsafe { self.data.get_unchecked(start..self.pos) })
    }

    /// Find the next `\r\n` starting from the current position.
    /// Returns the slice *before* the CRLF and advances past it.
    #[inline(always)]
    fn read_line(&mut self) -> Result<&'a [u8], ParseError> {
        let start = self.pos;
        let data = self.data;
        let len = data.len();

        // We search for '\n' and verify the preceding byte is '\r'.
        // In RESP, header lines are short so a tight scalar loop is optimal
        // (often < 16 bytes — well within L1 and branch-predictor range).
        let mut i = start + 1; // need at least one byte before \n
        while i < len {
            if unsafe { *data.get_unchecked(i) } == b'\n' {
                if unsafe { *data.get_unchecked(i - 1) } == b'\r' {
                    let line = unsafe { data.get_unchecked(start..i - 1) };
                    self.pos = i + 1;
                    return Ok(line);
                }
            }
            i += 1;
        }
        Err(ParseError::Incomplete)
    }

    /// Consume exactly `\r\n`.
    #[inline(always)]
    fn expect_crlf(&mut self) -> Result<(), ParseError> {
        if self.remaining() < 2 {
            return Err(ParseError::Incomplete);
        }
        let a = unsafe { *self.data.get_unchecked(self.pos) };
        let b = unsafe { *self.data.get_unchecked(self.pos + 1) };
        if a != b'\r' || b != b'\n' {
            return Err(ParseError::Protocol("expected CRLF"));
        }
        self.pos += 2;
        Ok(())
    }
}

/// Parse a non-negative integer from a byte slice (digits only).
#[inline(always)]
fn parse_uint(data: &[u8]) -> Result<usize, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Protocol("empty integer"));
    }
    let mut val: usize = 0;
    for &b in data {
        let d = b.wrapping_sub(b'0');
        if d > 9 {
            return Err(ParseError::Protocol("invalid digit"));
        }
        val = val.wrapping_mul(10).wrapping_add(d as usize);
    }
    Ok(val)
}

/// Parse a signed integer from a byte slice (optional leading `-`).
#[inline(always)]
fn parse_int(data: &[u8]) -> Result<i64, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Protocol("empty integer"));
    }
    let (neg, start) = if data[0] == b'-' {
        (true, 1)
    } else {
        (false, 0)
    };
    let mut val: i64 = 0;
    let mut i = start;
    while i < data.len() {
        let d = unsafe { *data.get_unchecked(i) }.wrapping_sub(b'0');
        if d > 9 {
            return Err(ParseError::Protocol("invalid digit in integer"));
        }
        val = val.wrapping_mul(10).wrapping_add(d as i64);
        i += 1;
    }
    Ok(if neg { -val } else { val })
}


// Supports:
//   • Array (RESP) commands:   *N\r\n $len\r\n data\r\n …
//   • Inline commands:         TOKEN TOKEN … \r\n

pub fn parse_command_into(
    src: &mut BytesMut,
    args: &mut Vec<CompactArg>,
) -> Result<Option<()>, ParseError> {
    args.clear();
    let data = &src[..];
    if data.is_empty() {
        return Ok(None);
    }

    let result = match data[0] {
        b'*' => parse_array_command(src, args),
        _ => parse_inline_command(src, args),
    };

    match result {
        Err(ParseError::Incomplete) => Ok(None),
        other => other,
    }
}

#[inline]
fn parse_array_command(
    src: &mut BytesMut,
    args: &mut Vec<CompactArg>,
) -> Result<Option<()>, ParseError> {
    let mut cur = Cursor::new(&src[..]);

    // skip '*'
    unsafe { cur.advance_unchecked(1) };

    // read argument count
    let count_line = cur.read_line()?;
    let count = parse_uint(count_line)?;

    if count == 0 {
        let consumed = cur.pos;
        let _ = src.split_to(consumed);
        return Ok(Some(()));
    }

    args.reserve(count);

    for _ in 0..count {
        // expect '$'
        match cur.peek() {
            Some(b'$') => {}
            Some(_) => return Err(ParseError::Protocol("expected '$' in array command")),
            None => return Err(ParseError::Incomplete),
        }
        unsafe { cur.advance_unchecked(1) };

        // read bulk length
        let len_line = cur.read_line()?;
        let bulk_len = parse_uint(len_line)?;

        // read payload
        let payload = cur.take(bulk_len)?;
        let arg = CompactArg::from_slice(payload);

        // CRLF after payload
        cur.expect_crlf()?;

        args.push(arg);
    }

    let consumed = cur.pos;
    let _ = src.split_to(consumed);
    Ok(Some(()))
}

#[inline]
fn parse_inline_command(
    src: &mut BytesMut,
    args: &mut Vec<CompactArg>,
) -> Result<Option<()>, ParseError> {
    let mut cur = Cursor::new(&src[..]);
    let line = cur.read_line()?;

    // Split line by spaces (consecutive spaces produce no empty tokens, per Redis spec).
    let mut i = 0;
    let len = line.len();
    while i < len {
        // skip whitespace
        while i < len && unsafe { *line.get_unchecked(i) } == b' ' {
            i += 1;
        }
        if i >= len {
            break;
        }
        let start = i;
        while i < len && unsafe { *line.get_unchecked(i) } != b' ' {
            i += 1;
        }
        args.push(CompactArg::from_slice(unsafe {
            line.get_unchecked(start..i)
        }));
    }

    if args.is_empty() {
        return Err(ParseError::Protocol("empty inline command"));
    }

    let consumed = cur.pos;
    let _ = src.split_to(consumed);
    Ok(Some(()))
}

pub fn parse_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
    let data = &src[..];
    if data.is_empty() {
        return Ok(None);
    }
    let mut cur = Cursor::new(data);
    match parse_frame_inner(&mut cur) {
        Ok(frame) => {
            let consumed = cur.pos;
            let _ = src.split_to(consumed);
            Ok(Some(frame))
        }
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

fn parse_frame_inner(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let tag = cur.peek().ok_or(ParseError::Incomplete)?;
    unsafe { cur.advance_unchecked(1) };

    match tag {
        b'+' => parse_simple(cur),
        b'-' => parse_error(cur),
        b':' => parse_integer(cur),
        b'$' => parse_bulk(cur),
        b'*' => parse_array(cur),
        b'%' => parse_map(cur),
        _ => Err(ParseError::Protocol("unknown RESP type tag")),
    }
}

#[inline]
fn parse_simple(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    // Must allocate because we don't own the bytes.
    let s = unsafe { String::from_utf8_unchecked(line.to_vec()) };
    Ok(RespFrame::Simple(s))
}

#[inline]
fn parse_error(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    let s = unsafe { String::from_utf8_unchecked(line.to_vec()) };
    Ok(RespFrame::Error(s))
}

#[inline]
fn parse_integer(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    let v = parse_int(line)?;
    Ok(RespFrame::Integer(v))
}

#[inline]
fn parse_bulk(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    // Null bulk: $-1\r\n
    if line.first() == Some(&b'-') {
        return Ok(RespFrame::Bulk(None));
    }

    let len = parse_uint(line)?;
    let payload = cur.take(len)?;

    // Decide Arg vs Value based on size.  ≤15 bytes → Arg (inline),
    // otherwise → Value.  This matches the CompactArg/CompactValue split.
    let bulk = if len <= 15 {
        BulkData::Arg(CompactArg::from_slice(payload))
    } else {
        BulkData::Value(CompactValue::from_slice(payload))
    };

    cur.expect_crlf()?;
    Ok(RespFrame::Bulk(Some(bulk)))
}

#[inline]
fn parse_array(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    // Null array: *-1\r\n
    if line.first() == Some(&b'-') {
        return Ok(RespFrame::Array(None));
    }

    let count = parse_uint(line)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(parse_frame_inner(cur)?);
    }
    Ok(RespFrame::Array(Some(items)))
}

#[inline]
fn parse_map(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    let count = parse_uint(line)?;
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        let key = parse_frame_inner(cur)?;
        let val = parse_frame_inner(cur)?;
        pairs.push((key, val));
    }
    Ok(RespFrame::Map(pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_command() {
        let mut src = BytesMut::from(&b"PING\r\n"[..]);
        let mut args = Vec::new();
        parse_command_into(&mut src, &mut args).unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].as_slice(), b"PING");
        assert!(src.is_empty());
    }

    #[test]
    fn inline_command_multi_args() {
        let mut src = BytesMut::from(&b"SET key value\r\n"[..]);
        let mut args = Vec::new();
        parse_command_into(&mut src, &mut args).unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].as_slice(), b"SET");
        assert_eq!(args[1].as_slice(), b"key");
        assert_eq!(args[2].as_slice(), b"value");
    }

    #[test]
    fn array_command() {
        let mut src = BytesMut::from(&b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"[..]);
        let mut args = Vec::new();
        parse_command_into(&mut src, &mut args).unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].as_slice(), b"SET");
        assert_eq!(args[1].as_slice(), b"key");
        assert_eq!(args[2].as_slice(), b"value");
        assert!(src.is_empty());
    }

    #[test]
    fn parse_simple_frame() {
        let mut src = BytesMut::from(&b"+OK\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        assert_eq!(frame, RespFrame::Simple("OK".into()));
    }

    #[test]
    fn parse_integer_frame() {
        let mut src = BytesMut::from(&b":-42\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        assert_eq!(frame, RespFrame::Integer(-42));
    }

    #[test]
    fn parse_bulk_frame() {
        let mut src = BytesMut::from(&b"$5\r\nhello\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        assert_eq!(
            frame,
            RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(b"hello"))))
        );
    }

    #[test]
    fn parse_null_bulk() {
        let mut src = BytesMut::from(&b"$-1\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        assert_eq!(frame, RespFrame::Bulk(None));
    }

    #[test]
    fn parse_array_frame() {
        let mut src = BytesMut::from(&b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        match frame {
            RespFrame::Array(Some(items)) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn incomplete_returns_error() {
        let mut src = BytesMut::from(&b"*3\r\n$3\r\nSET\r\n"[..]);
        let mut args = Vec::new();
        let result = parse_command_into(&mut src, &mut args);
        assert!(matches!(result, Err(ParseError::Incomplete)));
    }
}

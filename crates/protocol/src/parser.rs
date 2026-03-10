use bytes::{Buf, Bytes, BytesMut};
use smallvec::SmallVec;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}

impl ByteSpan {
    #[inline(always)]
    pub const fn len(self) -> usize {
        (self.end - self.start) as usize
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[inline(always)]
fn make_span(start: usize, end: usize) -> Result<ByteSpan, ParseError> {
    if end > u32::MAX as usize {
        return Err(ParseError::Protocol("frame too large"));
    }
    Ok(ByteSpan {
        start: start as u32,
        end: end as u32,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    raw: Bytes,
    args: SmallVec<[ByteSpan; 8]>,
}

impl ParsedCommand {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.args.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    #[inline(always)]
    pub fn arg(&self, index: usize) -> &[u8] {
        let span = unsafe { *self.args.get_unchecked(index) };
        unsafe {
            self.raw
                .as_ref()
                .get_unchecked(span.start as usize..span.end as usize)
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &[u8]> + '_ {
        self.args.iter().map(|span| unsafe {
            self.raw
                .as_ref()
                .get_unchecked(span.start as usize..span.end as usize)
        })
    }

    #[inline(always)]
    pub fn into_raw(self) -> Bytes {
        self.raw
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BorrowedFrame {
    Simple(ByteSpan),
    Error(ByteSpan),
    Integer(i64),
    Bulk(Option<ByteSpan>),
    Array(Option<Vec<BorrowedFrame>>),
    Map(Vec<(BorrowedFrame, BorrowedFrame)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedFrame {
    raw: Bytes,
    frame: BorrowedFrame,
}

impl ParsedFrame {
    #[inline(always)]
    pub fn frame(&self) -> &BorrowedFrame {
        &self.frame
    }

    #[inline(always)]
    pub fn bytes(&self, span: ByteSpan) -> &[u8] {
        unsafe {
            self.raw
                .as_ref()
                .get_unchecked(span.start as usize..span.end as usize)
        }
    }

    #[inline(always)]
    pub fn into_raw(self) -> Bytes {
        self.raw
    }
}

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

    #[inline(always)]
    unsafe fn advance_unchecked(&mut self, n: usize) {
        self.pos += n;
    }

    #[inline(always)]
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.remaining() < n {
            return Err(ParseError::Incomplete);
        }
        let start = self.pos;
        self.pos += n;
        Ok(unsafe { self.data.get_unchecked(start..self.pos) })
    }

    #[inline(always)]
    fn read_line(&mut self) -> Result<&'a [u8], ParseError> {
        let start = self.pos;
        let data = self.data;
        let len = data.len();

        let mut i = start + 1;
        while i < len {
            if unsafe { *data.get_unchecked(i) } == b'\n'
                && unsafe { *data.get_unchecked(i - 1) } == b'\r'
            {
                let line = unsafe { data.get_unchecked(start..i - 1) };
                self.pos = i + 1;
                return Ok(line);
            }
            i += 1;
        }

        Err(ParseError::Incomplete)
    }

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

#[inline(always)]
fn parse_digit_usize(b: u8) -> Result<usize, ParseError> {
    let d = b.wrapping_sub(b'0');
    if d > 9 {
        Err(ParseError::Protocol("invalid digit"))
    } else {
        Ok(d as usize)
    }
}

#[cold]
fn parse_uint_slow(data: &[u8]) -> Result<usize, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Protocol("empty integer"));
    }

    let mut val = 0usize;
    const MAX_PRE: usize = usize::MAX / 10;
    const MAX_LAST: usize = usize::MAX % 10;

    for &b in data {
        let d = b.wrapping_sub(b'0');
        if d > 9 {
            return Err(ParseError::Protocol("invalid digit"));
        }
        let d = d as usize;
        if val > MAX_PRE || (val == MAX_PRE && d > MAX_LAST) {
            return Err(ParseError::Protocol("integer overflow"));
        }
        val = val * 10 + d;
    }

    Ok(val)
}

#[inline(always)]
fn parse_uint(data: &[u8]) -> Result<usize, ParseError> {
    match data {
        [] => Err(ParseError::Protocol("empty integer")),
        [a] => parse_digit_usize(*a),
        [a, b] => Ok(parse_digit_usize(*a)? * 10 + parse_digit_usize(*b)?),
        [a, b, c] => {
            Ok(parse_digit_usize(*a)? * 100 + parse_digit_usize(*b)? * 10 + parse_digit_usize(*c)?)
        }
        [a, b, c, d] => Ok(parse_digit_usize(*a)? * 1000
            + parse_digit_usize(*b)? * 100
            + parse_digit_usize(*c)? * 10
            + parse_digit_usize(*d)?),
        _ => parse_uint_slow(data),
    }
}

#[inline(always)]
fn parse_int(data: &[u8]) -> Result<i64, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Protocol("empty integer"));
    }

    let (neg, digits) = if data[0] == b'-' {
        (true, &data[1..])
    } else {
        (false, data)
    };

    if digits.is_empty() {
        return Err(ParseError::Protocol("invalid integer"));
    }

    let mag = parse_uint(digits)? as u64;

    if neg {
        let limit = i64::MAX as u64 + 1;
        if mag > limit {
            return Err(ParseError::Protocol("integer overflow"));
        }
        if mag == limit {
            Ok(i64::MIN)
        } else {
            Ok(-(mag as i64))
        }
    } else {
        if mag > i64::MAX as u64 {
            return Err(ParseError::Protocol("integer overflow"));
        }
        Ok(mag as i64)
    }
}

pub fn parse_command_borrowed(src: &mut BytesMut) -> Result<Option<ParsedCommand>, ParseError> {
    let data = &src[..];
    if data.is_empty() {
        return Ok(None);
    }

    let result = match data[0] {
        b'*' => parse_array_command_ranges(data),
        _ => parse_inline_command_ranges(data),
    };

    match result {
        Ok((consumed, args)) => {
            let raw = src.split_to(consumed).freeze();
            Ok(Some(ParsedCommand { raw, args }))
        }
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

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
        b'*' => parse_array_command_owned(src, args),
        _ => parse_inline_command_owned(src, args),
    };

    match result {
        Err(ParseError::Incomplete) => Ok(None),
        other => other,
    }
}

#[inline(always)]
fn parse_array_command_ranges(data: &[u8]) -> Result<(usize, SmallVec<[ByteSpan; 8]>), ParseError> {
    let mut cur = Cursor::new(data);
    unsafe { cur.advance_unchecked(1) };

    let count_line = cur.read_line()?;
    let count = parse_uint(count_line)?;

    let mut args = SmallVec::<[ByteSpan; 8]>::with_capacity(count);

    for _ in 0..count {
        match cur.peek() {
            Some(b'$') => {}
            Some(_) => return Err(ParseError::Protocol("expected '$' in array command")),
            None => return Err(ParseError::Incomplete),
        }
        unsafe { cur.advance_unchecked(1) };

        let len_line = cur.read_line()?;
        let bulk_len = parse_uint(len_line)?;

        let start = cur.pos;
        cur.take(bulk_len)?;
        let end = cur.pos;
        cur.expect_crlf()?;

        args.push(make_span(start, end)?);
    }

    Ok((cur.pos, args))
}

#[inline(always)]
fn parse_inline_command_ranges(
    data: &[u8],
) -> Result<(usize, SmallVec<[ByteSpan; 8]>), ParseError> {
    let mut cur = Cursor::new(data);
    let line = cur.read_line()?;

    let base = line.as_ptr() as usize - data.as_ptr() as usize;
    let mut args = SmallVec::<[ByteSpan; 8]>::new();

    let mut i = 0usize;
    let len = line.len();

    while i < len {
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

        args.push(make_span(base + start, base + i)?);
    }

    if args.is_empty() {
        return Err(ParseError::Protocol("empty inline command"));
    }

    Ok((cur.pos, args))
}

#[inline(always)]
fn parse_array_command_owned(
    src: &mut BytesMut,
    args: &mut Vec<CompactArg>,
) -> Result<Option<()>, ParseError> {
    let mut cur = Cursor::new(&src[..]);
    unsafe { cur.advance_unchecked(1) };

    let count_line = cur.read_line()?;
    let count = parse_uint(count_line)?;

    if args.capacity() < count {
        args.reserve(count);
    }

    for _ in 0..count {
        match cur.peek() {
            Some(b'$') => {}
            Some(_) => return Err(ParseError::Protocol("expected '$' in array command")),
            None => return Err(ParseError::Incomplete),
        }
        unsafe { cur.advance_unchecked(1) };

        let len_line = cur.read_line()?;
        let bulk_len = parse_uint(len_line)?;
        let payload = cur.take(bulk_len)?;
        cur.expect_crlf()?;

        args.push(CompactArg::from_slice(payload));
    }

    src.advance(cur.pos);
    Ok(Some(()))
}

#[inline(always)]
fn parse_inline_command_owned(
    src: &mut BytesMut,
    args: &mut Vec<CompactArg>,
) -> Result<Option<()>, ParseError> {
    let mut cur = Cursor::new(&src[..]);
    let line = cur.read_line()?;

    let mut i = 0usize;
    let len = line.len();

    while i < len {
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

    src.advance(cur.pos);
    Ok(Some(()))
}

pub fn parse_frame_borrowed(src: &mut BytesMut) -> Result<Option<ParsedFrame>, ParseError> {
    let data = &src[..];
    if data.is_empty() {
        return Ok(None);
    }

    let mut cur = Cursor::new(data);
    match parse_frame_borrowed_inner(&mut cur) {
        Ok(frame) => {
            let raw = src.split_to(cur.pos).freeze();
            Ok(Some(ParsedFrame { raw, frame }))
        }
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn parse_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
    let data = &src[..];
    if data.is_empty() {
        return Ok(None);
    }

    let mut cur = Cursor::new(data);
    match parse_frame_inner(&mut cur) {
        Ok(frame) => {
            src.advance(cur.pos);
            Ok(Some(frame))
        }
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

#[inline(always)]
fn parse_frame_borrowed_inner(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let tag = cur.peek().ok_or(ParseError::Incomplete)?;
    unsafe { cur.advance_unchecked(1) };

    match tag {
        b'+' => parse_simple_borrowed(cur),
        b'-' => parse_error_borrowed(cur),
        b':' => parse_integer_borrowed(cur),
        b'$' => parse_bulk_borrowed(cur),
        b'*' => parse_array_borrowed(cur),
        b'%' => parse_map_borrowed(cur),
        _ => Err(ParseError::Protocol("unknown RESP type tag")),
    }
}

#[inline(always)]
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

#[inline(always)]
fn parse_simple_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let start = cur.pos;
    let line = cur.read_line()?;
    let end = start + line.len();
    Ok(BorrowedFrame::Simple(make_span(start, end)?))
}

#[inline(always)]
fn parse_error_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let start = cur.pos;
    let line = cur.read_line()?;
    let end = start + line.len();
    Ok(BorrowedFrame::Error(make_span(start, end)?))
}

#[inline(always)]
fn parse_integer_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let line = cur.read_line()?;
    Ok(BorrowedFrame::Integer(parse_int(line)?))
}

#[inline(always)]
fn parse_bulk_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let line = cur.read_line()?;

    if line == b"-1" {
        return Ok(BorrowedFrame::Bulk(None));
    }
    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid bulk length"));
    }

    let len = parse_uint(line)?;
    let start = cur.pos;
    cur.take(len)?;
    let end = cur.pos;
    cur.expect_crlf()?;

    Ok(BorrowedFrame::Bulk(Some(make_span(start, end)?)))
}

#[inline(always)]
fn parse_array_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let line = cur.read_line()?;

    if line == b"-1" {
        return Ok(BorrowedFrame::Array(None));
    }
    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid array length"));
    }

    let count = parse_uint(line)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(parse_frame_borrowed_inner(cur)?);
    }
    Ok(BorrowedFrame::Array(Some(items)))
}

#[inline(always)]
fn parse_map_borrowed(cur: &mut Cursor<'_>) -> Result<BorrowedFrame, ParseError> {
    let line = cur.read_line()?;

    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid map length"));
    }

    let count = parse_uint(line)?;
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        let key = parse_frame_borrowed_inner(cur)?;
        let val = parse_frame_borrowed_inner(cur)?;
        pairs.push((key, val));
    }
    Ok(BorrowedFrame::Map(pairs))
}

#[inline(always)]
fn parse_simple(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    match line {
        b"OK" => Ok(RespFrame::SimpleStatic("OK")),
        b"PONG" => Ok(RespFrame::SimpleStatic("PONG")),
        b"QUEUED" => Ok(RespFrame::SimpleStatic("QUEUED")),
        _ => {
            let s = std::str::from_utf8(line)
                .map_err(|_| ParseError::Protocol("invalid UTF-8 in simple string"))?
                .to_owned();
            Ok(RespFrame::Simple(s))
        }
    }
}

#[inline(always)]
fn parse_error(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    let s = std::str::from_utf8(line)
        .map_err(|_| ParseError::Protocol("invalid UTF-8 in error string"))?
        .to_owned();
    Ok(RespFrame::Error(s))
}

#[inline(always)]
fn parse_integer(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;
    Ok(RespFrame::Integer(parse_int(line)?))
}

#[inline(always)]
fn parse_bulk(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    if line == b"-1" {
        return Ok(RespFrame::Bulk(None));
    }
    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid bulk length"));
    }

    let len = parse_uint(line)?;
    let payload = cur.take(len)?;
    cur.expect_crlf()?;

    let bulk = if len <= 15 {
        BulkData::Arg(CompactArg::from_slice(payload))
    } else {
        BulkData::Value(CompactValue::from_slice(payload))
    };

    Ok(RespFrame::Bulk(Some(bulk)))
}

#[inline(always)]
fn parse_array(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    if line == b"-1" {
        return Ok(RespFrame::Array(None));
    }
    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid array length"));
    }

    let count = parse_uint(line)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(parse_frame_inner(cur)?);
    }
    Ok(RespFrame::Array(Some(items)))
}

#[inline(always)]
fn parse_map(cur: &mut Cursor<'_>) -> Result<RespFrame, ParseError> {
    let line = cur.read_line()?;

    if line.first() == Some(&b'-') {
        return Err(ParseError::Protocol("invalid map length"));
    }

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
    fn borrowed_inline_command() {
        let mut src = BytesMut::from(&b"SET key value PX 100\r\n"[..]);
        let cmd = parse_command_borrowed(&mut src).unwrap().unwrap();
        assert_eq!(cmd.len(), 5);
        assert_eq!(cmd.arg(0), b"SET");
        assert_eq!(cmd.arg(1), b"key");
        assert_eq!(cmd.arg(2), b"value");
        assert_eq!(cmd.arg(3), b"PX");
        assert_eq!(cmd.arg(4), b"100");
        assert!(src.is_empty());
    }

    #[test]
    fn borrowed_array_command() {
        let mut src = BytesMut::from(&b"*2\r\n$4\r\nLLEN\r\n$5\r\nqueue\r\n"[..]);
        let cmd = parse_command_borrowed(&mut src).unwrap().unwrap();
        assert_eq!(cmd.len(), 2);
        assert_eq!(cmd.arg(0), b"LLEN");
        assert_eq!(cmd.arg(1), b"queue");
        assert!(src.is_empty());
    }

    #[test]
    fn borrowed_bulk_frame() {
        let mut src = BytesMut::from(&b"$5\r\nhello\r\n"[..]);
        let frame = parse_frame_borrowed(&mut src).unwrap().unwrap();
        match frame.frame() {
            BorrowedFrame::Bulk(Some(span)) => assert_eq!(frame.bytes(*span), b"hello"),
            other => panic!("expected borrowed bulk, got {other:?}"),
        }
        assert!(src.is_empty());
    }

    #[test]
    fn parse_simple_frame() {
        let mut src = BytesMut::from(&b"+OK\r\n"[..]);
        let frame = parse_frame(&mut src).unwrap().unwrap();
        assert_eq!(frame, RespFrame::SimpleStatic("OK"));
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
        assert!(matches!(result, Ok(None)));
    }
}

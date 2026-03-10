use crate::types::RespFrame;
use bytes::BytesMut;

#[inline(always)]
fn digits_usize(mut v: usize) -> usize {
    let mut d = 1;
    while v >= 10 {
        v /= 10;
        d += 1;
    }
    d
}

#[inline(always)]
fn digits_i64(v: i64) -> usize {
    let mut n = v.unsigned_abs();
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    if v < 0 { d + 1 } else { d }
}

#[inline(always)]
fn write_int(buf: &mut BytesMut, val: i64) {
    let mut tmp = itoa::Buffer::new();
    buf.extend_from_slice(tmp.format(val).as_bytes());
}

#[inline(always)]
fn write_uint(buf: &mut BytesMut, val: usize) {
    let mut tmp = itoa::Buffer::new();
    buf.extend_from_slice(tmp.format(val).as_bytes());
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Encoder;

static CRLF: &[u8; 2] = b"\r\n";
static SIMPLE_OK: &[u8] = b"+OK\r\n";
static SIMPLE_PONG: &[u8] = b"+PONG\r\n";
static SIMPLE_QUEUED: &[u8] = b"+QUEUED\r\n";
static INTEGER_ZERO: &[u8] = b":0\r\n";
static INTEGER_ONE: &[u8] = b":1\r\n";
static NULL_BULK: &[u8] = b"$-1\r\n";
static NULL_ARRAY: &[u8] = b"*-1\r\n";

#[inline(always)]
fn bulk_encoded_len(len: usize) -> usize {
    1 + digits_usize(len) + 2 + len + 2
}

#[inline(always)]
fn encoded_len(frame: &RespFrame) -> usize {
    match frame {
        RespFrame::Simple(s) | RespFrame::Error(s) => 1 + s.len() + 2,
        RespFrame::SimpleStatic(s) | RespFrame::ErrorStatic(s) => 1 + s.len() + 2,
        RespFrame::Integer(v) => 1 + digits_i64(*v) + 2,
        RespFrame::Bulk(None) => NULL_BULK.len(),
        RespFrame::Bulk(Some(data)) => bulk_encoded_len(data.as_slice().len()),
        RespFrame::BulkOptions(values) => {
            1 + digits_usize(values.len())
                + 2
                + values
                    .iter()
                    .map(|value| match value {
                        Some(value) => bulk_encoded_len(value.as_slice().len()),
                        None => NULL_BULK.len(),
                    })
                    .sum::<usize>()
        }
        RespFrame::BulkValues(values) => {
            1 + digits_usize(values.len())
                + 2
                + values
                    .iter()
                    .map(|value| bulk_encoded_len(value.as_slice().len()))
                    .sum::<usize>()
        }
        RespFrame::PreEncoded(bytes) => bytes.len(),
        RespFrame::Array(None) => NULL_ARRAY.len(),
        RespFrame::Array(Some(items)) => {
            1 + digits_usize(items.len()) + 2 + items.iter().map(encoded_len).sum::<usize>()
        }
        RespFrame::Map(pairs) => {
            1 + digits_usize(pairs.len())
                + 2
                + pairs
                    .iter()
                    .map(|(key, val)| encoded_len(key) + encoded_len(val))
                    .sum::<usize>()
        }
    }
}

#[inline(always)]
fn write_bulk_slice(buf: &mut BytesMut, slice: &[u8]) {
    buf.extend_from_slice(b"$");
    write_uint(buf, slice.len());
    buf.extend_from_slice(CRLF);
    buf.extend_from_slice(slice);
    buf.extend_from_slice(CRLF);
}

impl Encoder {
    #[inline(always)]
    pub fn encode(&mut self, frame: &RespFrame, buf: &mut BytesMut) {
        buf.reserve(encoded_len(frame));
        self.encode_inner(frame, buf);
    }

    #[inline(always)]
    fn encode_inner(&mut self, frame: &RespFrame, buf: &mut BytesMut) {
        match frame {
            RespFrame::Simple(s) => {
                buf.extend_from_slice(b"+");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(CRLF);
            }
            RespFrame::SimpleStatic("OK") => buf.extend_from_slice(SIMPLE_OK),
            RespFrame::SimpleStatic("PONG") => buf.extend_from_slice(SIMPLE_PONG),
            RespFrame::SimpleStatic("QUEUED") => buf.extend_from_slice(SIMPLE_QUEUED),
            RespFrame::SimpleStatic(s) => {
                buf.extend_from_slice(b"+");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(CRLF);
            }
            RespFrame::Error(s) => {
                buf.extend_from_slice(b"-");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(CRLF);
            }
            RespFrame::ErrorStatic(s) => {
                buf.extend_from_slice(b"-");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(CRLF);
            }
            RespFrame::Integer(0) => buf.extend_from_slice(INTEGER_ZERO),
            RespFrame::Integer(1) => buf.extend_from_slice(INTEGER_ONE),
            RespFrame::Integer(v) => {
                buf.extend_from_slice(b":");
                write_int(buf, *v);
                buf.extend_from_slice(CRLF);
            }
            RespFrame::Bulk(None) => {
                buf.extend_from_slice(NULL_BULK);
            }
            RespFrame::Bulk(Some(data)) => {
                write_bulk_slice(buf, data.as_slice());
            }
            RespFrame::BulkOptions(values) => {
                buf.extend_from_slice(b"*");
                write_uint(buf, values.len());
                buf.extend_from_slice(CRLF);
                for value in values {
                    match value {
                        Some(value) => write_bulk_slice(buf, value.as_slice()),
                        None => buf.extend_from_slice(NULL_BULK),
                    }
                }
            }
            RespFrame::BulkValues(values) => {
                buf.extend_from_slice(b"*");
                write_uint(buf, values.len());
                buf.extend_from_slice(CRLF);
                for value in values {
                    write_bulk_slice(buf, value.as_slice());
                }
            }
            RespFrame::PreEncoded(bytes) => {
                buf.extend_from_slice(bytes.as_ref());
            }
            RespFrame::Array(None) => {
                buf.extend_from_slice(NULL_ARRAY);
            }
            RespFrame::Array(Some(items)) => {
                buf.extend_from_slice(b"*");
                write_uint(buf, items.len());
                buf.extend_from_slice(CRLF);
                for item in items {
                    self.encode_inner(item, buf);
                }
            }
            RespFrame::Map(pairs) => {
                buf.extend_from_slice(b"%");
                write_uint(buf, pairs.len());
                buf.extend_from_slice(CRLF);
                for (key, val) in pairs {
                    self.encode_inner(key, buf);
                    self.encode_inner(val, buf);
                }
            }
        }
    }
}

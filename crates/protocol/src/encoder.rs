use crate::types::RespFrame;
use bytes::BytesMut;

#[inline]
fn write_int(buf: &mut BytesMut, val: i64) {
    let mut tmp = [0u8; 20];
    let mut pos = 20usize;

    if val == 0 {
        buf.extend_from_slice(b"0");
        return;
    }

    let neg = val < 0;
    let mut v: u64 = if neg { (!(val as u64)).wrapping_add(1) } else { val as u64 };

    while v > 0 {
        pos -= 1;
        unsafe { *tmp.get_unchecked_mut(pos) = (v % 10) as u8 + b'0' };
        v /= 10;
    }
    if neg {
        pos -= 1;
        unsafe { *tmp.get_unchecked_mut(pos) = b'-' };
    }
    buf.extend_from_slice(unsafe { tmp.get_unchecked(pos..20) });
}

#[inline]
fn write_uint(buf: &mut BytesMut, val: usize) {
    let mut tmp = [0u8; 20];
    let mut pos = 20usize;

    if val == 0 {
        buf.extend_from_slice(b"0");
        return;
    }

    let mut v = val;
    while v > 0 {
        pos -= 1;
        unsafe { *tmp.get_unchecked_mut(pos) = (v % 10) as u8 + b'0' };
        v /= 10;
    }
    buf.extend_from_slice(unsafe { tmp.get_unchecked(pos..20) });
}

#[derive(Debug, Default, Clone)]
pub struct Encoder {}

static CRLF: &[u8; 2] = b"\r\n";

#[inline]
fn write_bulk_slice(buf: &mut BytesMut, slice: &[u8]) {
    buf.extend_from_slice(b"$");
    write_uint(buf, slice.len());
    buf.extend_from_slice(CRLF);
    buf.extend_from_slice(slice);
    buf.extend_from_slice(CRLF);
}

impl Encoder {
    #[inline]
    pub fn encode(&mut self, frame: &RespFrame, buf: &mut BytesMut) {
        match frame {
            RespFrame::Simple(s) => {
                buf.extend_from_slice(b"+");
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(CRLF);
            }
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
            RespFrame::Integer(v) => {
                buf.extend_from_slice(b":");
                write_int(buf, *v);
                buf.extend_from_slice(CRLF);
            }
            RespFrame::Bulk(None) => {
                buf.extend_from_slice(b"$-1\r\n");
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
                        None => buf.extend_from_slice(b"$-1\r\n"),
                    }
                }
            }
            RespFrame::BulkValues(values) => {
                buf.extend_from_slice(b"*");
                write_uint(buf, values.len());
                buf.extend_from_slice(CRLF);
                for v in values {
                    write_bulk_slice(buf, v.as_slice());
                }
            }
            RespFrame::PreEncoded(bytes) => {
                buf.extend_from_slice(bytes.as_ref());
            }
            RespFrame::Array(None) => {
                buf.extend_from_slice(b"*-1\r\n");
            }
            RespFrame::Array(Some(items)) => {
                buf.extend_from_slice(b"*");
                write_uint(buf, items.len());
                buf.extend_from_slice(CRLF);
                for item in items {
                    self.encode(item, buf);
                }
            }
            RespFrame::Map(pairs) => {
                buf.extend_from_slice(b"%");
                write_uint(buf, pairs.len());
                buf.extend_from_slice(CRLF);
                for (key, val) in pairs {
                    self.encode(key, buf);
                    self.encode(val, buf);
                }
            }
        }
    }
}

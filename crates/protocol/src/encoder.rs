use crate::types::RespFrame;
use bytes::{BufMut, BytesMut};

const CRLF: &[u8] = b"\r\n";
const NULL_BULK: &[u8] = b"$-1\r\n";
const NULL_ARRAY: &[u8] = b"*-1\r\n";

pub struct Encoder {
    itoa: itoa::Buffer,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            itoa: itoa::Buffer::new(),
        }
    }
}

impl Encoder {
    #[inline]
    pub fn encode(&mut self, frame: &RespFrame, out: &mut BytesMut) {
        let _trace = profiler::scope("protocol::encoder::encode");
        match frame {
            RespFrame::Simple(v) => {
                out.put_u8(b'+');
                out.put_slice(v.as_bytes());
                out.put_slice(CRLF);
            }
            RespFrame::SimpleStatic(v) => {
                out.put_u8(b'+');
                out.put_slice(v.as_bytes());
                out.put_slice(CRLF);
            }
            RespFrame::Error(v) => {
                out.put_u8(b'-');
                out.put_slice(v.as_bytes());
                out.put_slice(CRLF);
            }
            RespFrame::ErrorStatic(v) => {
                out.put_u8(b'-');
                out.put_slice(v.as_bytes());
                out.put_slice(CRLF);
            }
            RespFrame::Integer(n) => {
                out.put_u8(b':');
                out.put_slice(self.itoa.format(*n).as_bytes());
                out.put_slice(CRLF);
            }
            RespFrame::Bulk(None) => out.put_slice(NULL_BULK),
            RespFrame::Bulk(Some(v)) => self.encode_bulk_bytes(v.as_slice(), out),
            RespFrame::BulkValues(values) => {
                out.put_u8(b'*');
                out.put_slice(self.itoa.format(values.len()).as_bytes());
                out.put_slice(CRLF);
                for v in values {
                    self.encode_bulk_bytes(v.as_slice(), out);
                }
            }
            RespFrame::PreEncoded(bytes) => out.put_slice(bytes),
            RespFrame::Array(None) => out.put_slice(NULL_ARRAY),
            RespFrame::Array(Some(items)) => {
                out.put_u8(b'*');
                out.put_slice(self.itoa.format(items.len()).as_bytes());
                out.put_slice(CRLF);
                for item in items {
                    self.encode(item, out);
                }
            }
            RespFrame::Map(entries) => {
                out.put_u8(b'%');
                out.put_slice(self.itoa.format(entries.len()).as_bytes());
                out.put_slice(CRLF);
                for (k, v) in entries {
                    self.encode(k, out);
                    self.encode(v, out);
                }
            }
        }
    }

    #[inline]
    fn encode_bulk_bytes(&mut self, bytes: &[u8], out: &mut BytesMut) {
        out.put_u8(b'$');
        out.put_slice(self.itoa.format(bytes.len()).as_bytes());
        out.put_slice(CRLF);
        out.put_slice(bytes);
        out.put_slice(CRLF);
    }
}

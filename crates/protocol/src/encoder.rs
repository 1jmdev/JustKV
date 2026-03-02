use bytes::BytesMut;

use crate::types::RespFrame;

pub fn encode(frame: &RespFrame, out: &mut BytesMut) {
    match frame {
        RespFrame::Simple(value) => {
            out.extend_from_slice(b"+");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::SimpleStatic(value) => {
            out.extend_from_slice(b"+");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Error(value) => {
            out.extend_from_slice(b"-");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::ErrorStatic(value) => {
            out.extend_from_slice(b"-");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Integer(value) => {
            out.extend_from_slice(b":");
            push_i64(out, *value);
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Bulk(None) => out.extend_from_slice(b"$-1\r\n"),
        RespFrame::Bulk(Some(value)) => {
            out.extend_from_slice(b"$");
            let bytes = value.as_slice();
            push_usize(out, bytes.len());
            out.extend_from_slice(b"\r\n");
            out.extend_from_slice(bytes);
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::BulkValues(values) => {
            out.extend_from_slice(b"*");
            push_usize(out, values.len());
            out.extend_from_slice(b"\r\n");
            for value in values {
                out.extend_from_slice(b"$");
                let bytes = value.as_slice();
                push_usize(out, bytes.len());
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(bytes);
                out.extend_from_slice(b"\r\n");
            }
        }
        RespFrame::PreEncoded(bytes) => {
            out.extend_from_slice(bytes);
        }
        RespFrame::Array(None) => out.extend_from_slice(b"*-1\r\n"),
        RespFrame::Array(Some(items)) => {
            out.extend_from_slice(b"*");
            push_usize(out, items.len());
            out.extend_from_slice(b"\r\n");
            for item in items {
                encode(item, out);
            }
        }
        RespFrame::Map(entries) => {
            out.extend_from_slice(b"%");
            push_usize(out, entries.len());
            out.extend_from_slice(b"\r\n");
            for (key, value) in entries {
                encode(key, out);
                encode(value, out);
            }
        }
    }
}

fn push_i64(out: &mut BytesMut, value: i64) {
    let mut buf = itoa::Buffer::new();
    out.extend_from_slice(buf.format(value).as_bytes());
}

fn push_usize(out: &mut BytesMut, value: usize) {
    let mut buf = itoa::Buffer::new();
    out.extend_from_slice(buf.format(value).as_bytes());
}

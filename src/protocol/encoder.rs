use bytes::BytesMut;

use crate::protocol::types::RespFrame;

pub fn encode(frame: &RespFrame, out: &mut BytesMut) {
    match frame {
        RespFrame::Simple(value) => {
            out.extend_from_slice(b"+");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Error(value) => {
            out.extend_from_slice(b"-");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Integer(value) => {
            out.extend_from_slice(format!(":{value}\r\n").as_bytes());
        }
        RespFrame::Bulk(None) => out.extend_from_slice(b"$-1\r\n"),
        RespFrame::Bulk(Some(value)) => {
            out.extend_from_slice(format!("${}\r\n", value.len()).as_bytes());
            out.extend_from_slice(value);
            out.extend_from_slice(b"\r\n");
        }
        RespFrame::Array(None) => out.extend_from_slice(b"*-1\r\n"),
        RespFrame::Array(Some(items)) => {
            out.extend_from_slice(format!("*{}\r\n", items.len()).as_bytes());
            for item in items {
                encode(item, out);
            }
        }
        RespFrame::Map(entries) => {
            out.extend_from_slice(format!("%{}\r\n", entries.len()).as_bytes());
            for (key, value) in entries {
                encode(key, out);
                encode(value, out);
            }
        }
    }
}

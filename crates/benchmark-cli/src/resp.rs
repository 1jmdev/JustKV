use bytes::{Buf, BytesMut};
use protocol::parser::{self, ParseError};
use protocol::types::RespFrame;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub fn encode_resp_parts(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|part| part.len() + 16).sum::<usize>() + 16);
    out.push(b'*');
    append_u64(&mut out, parts.len() as u64);
    out.extend_from_slice(b"\r\n");

    for part in parts {
        out.push(b'$');
        append_u64(&mut out, part.len() as u64);
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
    out
}

pub fn make_key(base: &[u8], sequence: u64) -> Vec<u8> {
    if sequence == 0 {
        return base.to_vec();
    }

    let mut key = Vec::with_capacity(base.len() + 1 + 20);
    key.extend_from_slice(base);
    key.push(b':');
    append_u64(&mut key, sequence);
    key
}

pub fn repeat_payload(one: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(one.len() * count);
    for _ in 0..count {
        out.extend_from_slice(one);
    }
    out
}

pub async fn read_n_responses(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    expected: usize,
) -> Result<(), String> {
    let mut parsed = 0usize;
    let mut chunk = [0u8; 8192];

    while parsed < expected {
        loop {
            if parsed >= expected {
                return Ok(());
            }
            match parser::parse_frame(parse_buf) {
                Ok(Some(frame)) => {
                    if let RespFrame::Error(message) = frame {
                        return Err(format!("server returned error: {message}"));
                    }
                    parsed += 1;
                }
                Ok(None) | Err(ParseError::Incomplete) => break,
                Err(ParseError::Protocol(err)) => {
                    return Err(format!("protocol error: {err}"));
                }
            }
        }

        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
        parse_buf.extend_from_slice(&chunk[..read]);
    }

    Ok(())
}

pub async fn read_n_fixed_mget_responses(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    expected: usize,
    value_len: usize,
) -> Result<(), String> {
    let mut remaining = expected;
    let mut chunk = [0u8; 8192];
    let digits = decimal_len(value_len);
    let frame_len = 4 + 2 * (1 + digits + 2 + value_len + 2);

    while remaining > 0 {
        let available = parse_buf.len() / frame_len;
        if available > 0 {
            let consumed = available.min(remaining) * frame_len;
            parse_buf.advance(consumed);
            remaining -= available.min(remaining);
            continue;
        }

        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
        parse_buf.extend_from_slice(&chunk[..read]);
    }

    Ok(())
}

fn append_u64(out: &mut Vec<u8>, value: u64) {
    let mut tmp = itoa::Buffer::new();
    out.extend_from_slice(tmp.format(value).as_bytes());
}

fn decimal_len(value: usize) -> usize {
    if value < 10 {
        1
    } else if value < 100 {
        2
    } else if value < 1_000 {
        3
    } else if value < 10_000 {
        4
    } else {
        let mut n = value;
        let mut digits = 0;
        while n != 0 {
            digits += 1;
            n /= 10;
        }
        digits
    }
}

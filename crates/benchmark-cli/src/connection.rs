use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

use crate::cli::Config;
use crate::command::{CommandState, CommandTemplate};

pub struct RedisConnection {
    stream: Stream,
    read_buf: BytesMut,
}

enum Stream {
    Tcp(TcpStream),
    Unix(UnixStream),
}

impl RedisConnection {
    pub async fn connect(config: &Config) -> Result<Self, String> {
        let stream = if let Some(path) = &config.socket {
            Stream::Unix(UnixStream::connect(path).await.map_err(|err| format!("Unix socket connection failed: {err}"))?)
        } else {
            let address = format!("{}:{}", config.host, config.port);
            Stream::Tcp(TcpStream::connect(address).await.map_err(|err| format!("Connection failed: {err}"))?)
        };

        let mut connection = Self { stream, read_buf: BytesMut::with_capacity(8192) };
        let mut state = CommandState::new(0, 0);
        let mut buf = BytesMut::with_capacity(256);

        if config.resp3 {
            state.encode(&CommandTemplate::Resp(vec![b("HELLO"), b("3")]), None, &mut buf);
            connection.write_and_drain(&buf, 1).await?;
            buf.clear();
        }
        if let Some(password) = &config.password {
            let mut auth = vec![b("AUTH")];
            if let Some(user) = &config.user {
                auth.push(b(user.as_str()));
            }
            auth.push(b(password.as_str()));
            state.encode(&CommandTemplate::Resp(auth), None, &mut buf);
            connection.write_and_drain(&buf, 1).await?;
            buf.clear();
        }
        if config.dbnum != 0 {
            state.encode(&CommandTemplate::Resp(vec![b("SELECT"), b(config.dbnum.to_string())]), None, &mut buf);
            connection.write_and_drain(&buf, 1).await?;
            buf.clear();
        }
        if config.enable_tracking {
            state.encode(&CommandTemplate::Resp(vec![b("CLIENT"), b("TRACKING"), b("ON")]), None, &mut buf);
            connection.write_and_drain(&buf, 1).await?;
        }

        Ok(connection)
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> Result<(), String> {
        match &mut self.stream {
            Stream::Tcp(stream) => stream.write_all(buf).await,
            Stream::Unix(stream) => stream.write_all(buf).await,
        }
        .map_err(|err| format!("Write failed: {err}"))
    }

    pub async fn read_responses(&mut self, responses: usize) -> Result<usize, String> {
        let mut errors = 0;
        let mut remaining = responses;

        while remaining != 0 {
            match skip_frame(&self.read_buf) {
                Ok(Some((consumed, is_error))) => {
                    self.read_buf.advance(consumed);
                    remaining -= 1;
                    errors += usize::from(is_error);
                    continue;
                }
                Ok(None) => {}
                Err(err) => return Err(err),
            }

            let mut chunk = [0u8; 8192];
            let read = match &mut self.stream {
                Stream::Tcp(stream) => stream.read(&mut chunk).await,
                Stream::Unix(stream) => stream.read(&mut chunk).await,
            }
            .map_err(|err| format!("Read failed: {err}"))?;

            if read == 0 {
                return Err("Connection closed".to_string());
            }
            self.read_buf.extend_from_slice(&chunk[..read]);
        }

        Ok(errors)
    }

    pub async fn write_and_drain(&mut self, buf: &[u8], responses: usize) -> Result<(), String> {
        self.write_all(buf).await?;
        let errors = self.read_responses(responses).await?;
        if errors != 0 {
            return Err(format!("Redis error replies: {errors}"));
        }
        Ok(())
    }
}

fn skip_frame(src: &[u8]) -> Result<Option<(usize, bool)>, String> {
    if src.is_empty() {
        return Ok(None);
    }
    skip_value(src, 0).map(|value| value.map(|(consumed, is_error)| (consumed, is_error)))
}

fn skip_value(src: &[u8], offset: usize) -> Result<Option<(usize, bool)>, String> {
    if offset >= src.len() {
        return Ok(None);
    }

    match src[offset] {
        b'+' | b':' | b',' | b'#' | b'_' => skip_line(src, offset + 1, false),
        b'-' => skip_line(src, offset + 1, true),
        b'$' | b'!' | b'=' => skip_bulk(src, offset + 1, false),
        b'*' | b'~' | b'>' => skip_aggregate(src, offset + 1, 1),
        b'%' => skip_aggregate(src, offset + 1, 2),
        b'|' => skip_aggregate(src, offset + 1, 2),
        _ => Err("Protocol error: unsupported RESP type".to_string()),
    }
}

fn skip_line(src: &[u8], from: usize, is_error: bool) -> Result<Option<(usize, bool)>, String> {
    match find_crlf(src, from) {
        Some(end) => Ok(Some((end + 2, is_error))),
        None => Ok(None),
    }
}

fn skip_bulk(src: &[u8], from: usize, is_error: bool) -> Result<Option<(usize, bool)>, String> {
    let Some(end) = find_crlf(src, from) else {
        return Ok(None);
    };
    let len = parse_decimal(&src[from..end])?;
    if len < 0 {
        return Ok(Some((end + 2, is_error)));
    }
    let end_of_payload = end + 2 + len as usize;
    if src.len() < end_of_payload + 2 {
        return Ok(None);
    }
    if src[end_of_payload] != b'\r' || src[end_of_payload + 1] != b'\n' {
        return Err("Protocol error: missing bulk terminator".to_string());
    }
    Ok(Some((end_of_payload + 2, is_error)))
}

fn skip_aggregate(src: &[u8], from: usize, multiplier: usize) -> Result<Option<(usize, bool)>, String> {
    let Some(end) = find_crlf(src, from) else {
        return Ok(None);
    };
    let len = parse_decimal(&src[from..end])?;
    if len < 0 {
        return Ok(Some((end + 2, false)));
    }

    let mut cursor = end + 2;
    for _ in 0..(len as usize * multiplier) {
        let Some((consumed, _)) = skip_value(src, cursor)? else {
            return Ok(None);
        };
        cursor = consumed;
    }
    Ok(Some((cursor, false)))
}

fn find_crlf(src: &[u8], from: usize) -> Option<usize> {
    let mut index = from;
    while index + 1 < src.len() {
        if src[index] == b'\r' && src[index + 1] == b'\n' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_decimal(raw: &[u8]) -> Result<i64, String> {
    if raw.is_empty() {
        return Err("Protocol error: invalid integer".to_string());
    }

    let mut index = 0;
    let mut negative = false;
    if raw[0] == b'-' {
        negative = true;
        index = 1;
    }
    if index >= raw.len() {
        return Err("Protocol error: invalid integer".to_string());
    }

    let mut value: i64 = 0;
    while index < raw.len() {
        let digit = raw[index].wrapping_sub(b'0');
        if digit > 9 {
            return Err("Protocol error: invalid integer".to_string());
        }
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(i64::from(digit)))
            .ok_or_else(|| "Protocol error: integer overflow".to_string())?;
        index += 1;
    }

    if negative {
        value
            .checked_neg()
            .ok_or_else(|| "Protocol error: integer overflow".to_string())
    } else {
        Ok(value)
    }
}

fn b<T: Into<Vec<u8>>>(value: T) -> super::command::ArgTemplate {
    super::command::ArgTemplate::Static(value.into())
}

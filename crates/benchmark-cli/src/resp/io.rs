use bytes::{Buf, BytesMut};
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use super::ExpectedResponse;
use super::skip::try_skip_frame;

pub async fn consume_response(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    expected: Option<&ExpectedResponse>,
    encoded: Option<&[u8]>,
    strict: bool,
) -> Result<(), String> {
    if strict {
        if let Some(encoded) = encoded {
            return validate_exact_response(stream, parse_buf, encoded).await;
        }
        if let Some(expected) = expected {
            let frame = read_one_response(stream, parse_buf).await?;
            return validate_response(expected, &frame);
        }
    }

    if let Some(encoded) = encoded {
        return skip_exact_response(stream, parse_buf, encoded).await;
    }

    skip_one_response(stream, parse_buf).await
}

pub async fn read_one_response(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
) -> Result<RespFrame, String> {
    loop {
        match parser::parse_frame(parse_buf) {
            Ok(Some(frame)) => return Ok(frame),
            Ok(None) | Err(ParseError::Incomplete) => {}
            Err(ParseError::Protocol(err)) => return Err(format!("protocol error: {err}")),
        }

        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }
}

async fn skip_exact_response(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    expected: &[u8],
) -> Result<(), String> {
    while parse_buf.len() < expected.len() {
        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }

    if parse_buf.first().copied() == Some(b'-') {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("server returned error while skipping: {frame:?}"));
    }

    parse_buf.advance(expected.len());
    Ok(())
}

async fn validate_exact_response(
    stream: &mut TcpStream,
    parse_buf: &mut BytesMut,
    expected: &[u8],
) -> Result<(), String> {
    while parse_buf.len() < expected.len() {
        let read = stream
            .read_buf(parse_buf)
            .await
            .map_err(|err| format!("read failed: {err}"))?;
        if read == 0 {
            return Err("connection closed by server".to_string());
        }
    }

    if parse_buf.first().copied() == Some(b'-') {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("server returned error while validating: {frame:?}"));
    }

    if &parse_buf[..expected.len()] != expected {
        let frame = read_one_response(stream, parse_buf).await?;
        return Err(format!("unexpected response bytes, got {frame:?}"));
    }

    parse_buf.advance(expected.len());
    Ok(())
}

async fn skip_one_response(stream: &mut TcpStream, parse_buf: &mut BytesMut) -> Result<(), String> {
    loop {
        match try_skip_frame(parse_buf)? {
            Some(()) => return Ok(()),
            None => {
                let read = stream
                    .read_buf(parse_buf)
                    .await
                    .map_err(|err| format!("read failed: {err}"))?;
                if read == 0 {
                    return Err("connection closed by server".to_string());
                }
            }
        }
    }
}

fn validate_response(expected: &ExpectedResponse, actual: &RespFrame) -> Result<(), String> {
    match (expected, actual) {
        (ExpectedResponse::Simple(expected), RespFrame::Simple(actual)) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected simple {expected:?}, got {actual:?}"))
            }
        }
        (ExpectedResponse::Simple(expected), RespFrame::SimpleStatic(actual)) => {
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected simple {expected:?}, got {actual:?}"))
            }
        }
        (ExpectedResponse::Bulk(None), RespFrame::Bulk(None)) => Ok(()),
        (ExpectedResponse::Bulk(Some(expected)), RespFrame::Bulk(Some(BulkData::Arg(actual)))) => {
            validate_bytes(expected, actual.as_slice())
        }
        (
            ExpectedResponse::Bulk(Some(expected)),
            RespFrame::Bulk(Some(BulkData::Value(actual))),
        ) => validate_bytes(expected, actual.as_slice()),
        (ExpectedResponse::Array(expected_items), RespFrame::Array(Some(actual_items))) => {
            if expected_items.len() != actual_items.len() {
                return Err(format!(
                    "expected array len {}, got {}",
                    expected_items.len(),
                    actual_items.len()
                ));
            }
            for (expected_item, actual_item) in expected_items.iter().zip(actual_items.iter()) {
                validate_response(expected_item, actual_item)?;
            }
            Ok(())
        }
        _ => Err(format!(
            "unexpected response shape: expected {expected:?}, got {actual:?}"
        )),
    }
}

fn validate_bytes(expected: &[u8], actual: &[u8]) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected bulk {:?}, got {:?}",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(actual)
        ))
    }
}

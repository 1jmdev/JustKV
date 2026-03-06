use std::time::{Duration, Instant};

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use protocol::encoder::Encoder;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};

use crate::command::format_resp;

pub fn find_free_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    Some(listener.local_addr().ok()?.port())
}

pub async fn wait_for_server(port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "embedded server on :{port} did not become ready within {}s",
                timeout.as_secs()
            ));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

pub async fn send_recv(stream: &mut TcpStream, argv: &[Vec<u8>]) -> Result<(u64, String), String> {
    let frame = RespFrame::Array(Some(
        argv.iter()
            .map(|p| RespFrame::Bulk(Some(BulkData::from_vec(p.clone()))))
            .collect(),
    ));

    let mut out = BytesMut::with_capacity(256);
    let mut encoder = Encoder::default();
    encoder.encode(&frame, &mut out);

    let t0 = Instant::now();
    stream
        .write_all(&out)
        .await
        .map_err(|e| format!("write: {e}"))?;

    let resp = read_one_frame(stream).await?;
    let rtt_ns = t0.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;

    Ok((rtt_ns, format_resp(&resp)))
}

async fn read_one_frame(stream: &mut TcpStream) -> Result<RespFrame, String> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parser::parse_frame(&mut buf) {
            Ok(Some(frame)) => return Ok(frame),
            Ok(None) | Err(ParseError::Incomplete) => {}
            Err(ParseError::Protocol(e)) => return Err(format!("protocol error: {e}")),
        }

        let mut chunk = [0u8; 4096];
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            return Err("connection closed".to_string());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

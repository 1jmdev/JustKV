use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use valkey::config::Config;
use valkey::net::listener::run_listener;
use valkey::protocol::parser::parse_frame;
use valkey::protocol::types::RespFrame;

static NEXT_PORT: AtomicU16 = AtomicU16::new(19000);

pub async fn spawn_server() -> JoinHandle<()> {
    let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
    let config = Config {
        bind: "127.0.0.1".to_string(),
        port,
        shards: 8,
        sweep_interval_ms: 50,
    };

    let handle = tokio::spawn(async move {
        let _ = run_listener(config).await;
    });

    tokio::time::sleep(Duration::from_millis(60)).await;
    handle
}

pub fn current_port() -> u16 {
    NEXT_PORT.load(Ordering::Relaxed) - 1
}

pub async fn connect() -> TcpStream {
    TcpStream::connect(("127.0.0.1", current_port()))
        .await
        .expect("connect to server")
}

pub async fn send_command(stream: &mut TcpStream, parts: &[&[u8]]) -> RespFrame {
    let mut payload = Vec::new();
    payload.extend_from_slice(format!("*{}\r\n", parts.len()).as_bytes());
    for part in parts {
        payload.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        payload.extend_from_slice(part);
        payload.extend_from_slice(b"\r\n");
    }

    stream.write_all(&payload).await.expect("write command");
    read_frame(stream).await
}

async fn read_frame(stream: &mut TcpStream) -> RespFrame {
    let mut buf = BytesMut::with_capacity(1024);

    loop {
        if let Some(frame) = parse_frame(&mut buf).expect("parse frame") {
            return frame;
        }

        let read = stream.read_buf(&mut buf).await.expect("read frame bytes");
        assert!(read > 0, "server closed before sending a response");
    }
}

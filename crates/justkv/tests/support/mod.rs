use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use justkv::config::Config;
use justkv::net::listener::run_listener;
use justkv::protocol::parser::parse_frame;
use justkv::protocol::types::RespFrame;

static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

pub async fn spawn_server() -> (JoinHandle<()>, u16) {
    let port = next_port();
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
    (handle, port)
}

fn next_port() -> u16 {
    let current = NEXT_PORT.load(Ordering::Relaxed);
    if current == 0 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("unix epoch")
            .subsec_nanos() as u32;
        let base = 20000 + ((std::process::id() + now) % 20000) as u16;
        let _ = NEXT_PORT.compare_exchange(0, base, Ordering::SeqCst, Ordering::SeqCst);
    }

    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

pub async fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port))
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

pub async fn read_frame(stream: &mut TcpStream) -> RespFrame {
    let mut buf = BytesMut::with_capacity(1024);

    loop {
        if let Some(frame) = parse_frame(&mut buf).expect("parse frame") {
            return frame;
        }

        let read = stream.read_buf(&mut buf).await.expect("read frame bytes");
        assert!(read > 0, "server closed before sending a response");
    }
}

use bytes::BytesMut;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

use protocol::encoder::Encoder;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};

use crate::cli::{ConnectionOptions, ConnectionTarget};

enum ClientStream {
    Tcp(TcpStream),
    Unix(UnixStream),
}

pub struct Client {
    stream: ClientStream,
    read_buf: BytesMut,
    encoder: Encoder,
}

impl Client {
    pub async fn connect(options: &ConnectionOptions) -> Result<Self, String> {
        let stream = match &options.target {
            ConnectionTarget::Tcp { host, port } => {
                let address = format!("{host}:{port}");
                let stream = TcpStream::connect(address.as_str())
                    .await
                    .map_err(|err| format!("Connection error: {err}"))?;
                ClientStream::Tcp(stream)
            }
            ConnectionTarget::Unix { path } => {
                let stream = UnixStream::connect(path)
                    .await
                    .map_err(|err| format!("Connection error: {err}"))?;
                ClientStream::Unix(stream)
            }
        };

        let mut client = Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
            encoder: Encoder::default(),
        };

        if options.proto == 3 {
            let _ = client
                .execute(parts(vec!["HELLO", "3"]))
                .await
                .map_err(|err| format!("HELLO failed: {err}"))?;
        }

        if let Some(password) = options.password.as_deref() {
            let mut auth = vec!["AUTH".to_string()];
            if let Some(user) = options.user.as_deref() {
                auth.push(user.to_string());
            }
            auth.push(password.to_string());
            let _ = client
                .execute(strings_to_bytes(auth))
                .await
                .map_err(|err| format!("AUTH failed: {err}"))?;
        }

        if options.db != 0 {
            let _ = client
                .execute(parts(vec!["SELECT", &options.db.to_string()]))
                .await
                .map_err(|err| format!("SELECT failed: {err}"))?;
        }

        Ok(client)
    }

    pub async fn execute(&mut self, command: Vec<Vec<u8>>) -> Result<RespFrame, String> {
        let frame = RespFrame::Array(Some(
            command
                .into_iter()
                .map(|item| RespFrame::Bulk(Some(BulkData::from_vec(item))))
                .collect(),
        ));

        let mut out = BytesMut::with_capacity(256);
        self.encoder.encode(&frame, &mut out);
        self.stream
            .write_all(&out)
            .await
            .map_err(|err| format!("Write error: {err}"))?;
        self.read_frame().await
    }

    pub async fn execute_timed(&mut self, command: Vec<Vec<u8>>) -> Result<TimedResponse, String> {
        let started_at = Instant::now();
        let response = self.execute(command).await?;
        Ok(TimedResponse {
            response,
            duration: started_at.elapsed(),
        })
    }

    async fn read_frame(&mut self) -> Result<RespFrame, String> {
        loop {
            match parser::parse_frame(&mut self.read_buf) {
                Ok(Some(frame)) => return Ok(frame),
                Ok(None) => {}
                Err(ParseError::Protocol(err)) => {
                    return Err(format!("Protocol error: {err}"));
                }
                Err(ParseError::Incomplete) => {}
            }

            let mut chunk = [0u8; 4096];
            let size = self
                .stream
                .read(&mut chunk)
                .await
                .map_err(|err| format!("Read error: {err}"))?;

            if size == 0 {
                return Err("Connection closed".to_string());
            }

            self.read_buf.extend_from_slice(&chunk[..size]);
        }
    }
}

impl ClientStream {
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.write_all(buf).await,
            Self::Unix(stream) => stream.write_all(buf).await,
        }
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buf).await,
            Self::Unix(stream) => stream.read(buf).await,
        }
    }
}

pub struct TimedResponse {
    pub response: RespFrame,
    pub duration: Duration,
}

fn strings_to_bytes(parts: Vec<String>) -> Vec<Vec<u8>> {
    parts.into_iter().map(String::into_bytes).collect()
}

fn parts(parts: Vec<&str>) -> Vec<Vec<u8>> {
    parts
        .into_iter()
        .map(|value| value.as_bytes().to_vec())
        .collect()
}

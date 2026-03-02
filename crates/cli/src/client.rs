use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use protocol::encoder;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};

use crate::cli::ConnectionOptions;

pub struct Client {
    stream: TcpStream,
    read_buf: BytesMut,
}

impl Client {
    pub async fn connect(options: &ConnectionOptions) -> Result<Self, String> {
        let address = format!("{}:{}", options.host, options.port);
        let stream = TcpStream::connect(address.as_str())
            .await
            .map_err(|err| format!("Connection error: {err}"))?;

        let mut client = Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
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
        encoder::encode(&frame, &mut out);
        self.stream
            .write_all(&out)
            .await
            .map_err(|err| format!("Write error: {err}"))?;
        self.read_frame().await
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

fn strings_to_bytes(parts: Vec<String>) -> Vec<Vec<u8>> {
    parts.into_iter().map(String::into_bytes).collect()
}

fn parts(parts: Vec<&str>) -> Vec<Vec<u8>> {
    parts
        .into_iter()
        .map(|value| value.as_bytes().to_vec())
        .collect()
}

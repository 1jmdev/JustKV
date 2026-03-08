use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use protocol::encoder::Encoder;
use protocol::parser::{self, ParseError};
use protocol::types::{BulkData, RespFrame};

use crate::args::Args;
use crate::syntax::parse_command_line;

pub struct Client {
    stream: TcpStream,
    read_buf: BytesMut,
    encoder: Encoder,
}

impl Client {
    pub async fn connect(args: &Args) -> Result<Self, String> {
        let address = format!("{}:{}", args.host, args.port);
        let stream = TcpStream::connect(&address)
            .await
            .map_err(|err| format!("connection error to {address}: {err}"))?;

        stream
            .set_nodelay(true)
            .map_err(|err| format!("failed to enable TCP_NODELAY: {err}"))?;

        let mut client = Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
            encoder: Encoder::default(),
        };

        if let Some(password) = args.password.as_deref() {
            let mut auth = vec![b"AUTH".to_vec()];
            if let Some(user) = args.user.as_deref() {
                auth.push(user.as_bytes().to_vec());
            }
            auth.push(password.as_bytes().to_vec());
            client
                .execute(auth)
                .await
                .map_err(|err| format!("AUTH failed: {err}"))?;
        }

        if args.db != 0 {
            client
                .execute(vec![b"SELECT".to_vec(), args.db.to_string().into_bytes()])
                .await
                .map_err(|err| format!("SELECT failed: {err}"))?;
        }

        Ok(client)
    }

    pub async fn flush_all(&mut self) -> Result<(), String> {
        self.execute(vec![b"FLUSHALL".to_vec()]).await?;
        Ok(())
    }

    pub async fn execute_raw(&mut self, command: &str) -> Result<RespFrame, String> {
        let parts = parse_command_line(command)?;
        self.execute(parts).await
    }

    pub async fn execute_raw_no_reply(&mut self, command: &str) -> Result<(), String> {
        let parts = parse_command_line(command)?;
        self.send(parts).await
    }

    pub async fn execute(&mut self, command: Vec<Vec<u8>>) -> Result<RespFrame, String> {
        self.send(command).await?;
        self.read_frame().await
    }

    async fn send(&mut self, command: Vec<Vec<u8>>) -> Result<(), String> {
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
            .map_err(|err| format!("write error: {err}"))?;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<RespFrame, String> {
        loop {
            match parser::parse_frame(&mut self.read_buf) {
                Ok(Some(frame)) => return Ok(frame),
                Ok(None) | Err(ParseError::Incomplete) => {}
                Err(ParseError::Protocol(err)) => return Err(format!("protocol error: {err}")),
            }

            let mut chunk = [0u8; 4096];
            let size = self
                .stream
                .read(&mut chunk)
                .await
                .map_err(|err| format!("read error: {err}"))?;

            if size == 0 {
                return Err("connection closed".to_string());
            }

            self.read_buf.extend_from_slice(&chunk[..size]);
        }
    }
}

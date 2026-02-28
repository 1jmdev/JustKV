use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::engine::store::Store;
use crate::net::transaction::TransactionState;
use crate::protocol::encoder::encode;
use crate::protocol::parser::{ParseError, parse_frame};
use crate::protocol::types::RespFrame;

const READ_BUFFER_CAPACITY: usize = 256 * 1024;
const WRITE_BUFFER_CAPACITY: usize = 256 * 1024;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_CAPACITY);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_CAPACITY);
    let mut tx_state = TransactionState::default();

    loop {
        let bytes_read = stream.read_buf(&mut read_buf).await?;
        if bytes_read == 0 {
            break;
        }

        while let Some(frame) = parse_next_frame(&mut read_buf)? {
            let response = tx_state.handle_frame(&store, frame);
            encode(&response, &mut write_buf);
        }

        if !write_buf.is_empty() {
            stream.write_all(&write_buf).await?;
            write_buf.clear();
        }
    }

    Ok(())
}

fn parse_next_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
    match parse_frame(src) {
        Ok(frame) => Ok(frame),
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

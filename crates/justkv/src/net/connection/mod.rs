use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;

use crate::engine::store::Store;
use crate::net::pubsub::{ConnectionPubSub, PubSubHub};
use crate::net::transaction::TransactionState;
use crate::protocol::encoder::encode;
use crate::protocol::parser::{ParseError, parse_frame};
use crate::protocol::types::RespFrame;

mod dispatch;
mod notifications;
mod util;

const READ_BUFFER_CAPACITY: usize = 256 * 1024;
const WRITE_BUFFER_CAPACITY: usize = 256 * 1024;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_CAPACITY);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_CAPACITY);
    let mut tx_state = TransactionState::default();

    let (push_tx, mut push_rx) = unbounded_channel::<RespFrame>();
    let mut pubsub_state = ConnectionPubSub::new(pubsub_hub.next_connection_id());

    let result = loop {
        tokio::select! {
            push = push_rx.recv() => {
                let Some(frame) = push else {
                    break Ok(());
                };
                encode(&frame, &mut write_buf);
                if !write_buf.is_empty() {
                    if let Err(err) = stream.write_all(&write_buf).await {
                        break Err(err.into());
                    }
                    write_buf.clear();
                }
            }
            read_result = stream.read_buf(&mut read_buf) => {
                let bytes_read = match read_result {
                    Ok(value) => value,
                    Err(err) => break Err(err.into()),
                };
                if bytes_read == 0 {
                    break Ok(());
                }

                while let Some(frame) = parse_next_frame(&mut read_buf)? {
                    let response = tx_state.handle_frame_with(&store, frame, |store, frame| {
                        dispatch::execute_regular_command(
                            store,
                            &pubsub_hub,
                            &push_tx,
                            &mut pubsub_state,
                            frame,
                        )
                    });
                    encode(&response, &mut write_buf);
                }

                if !write_buf.is_empty() {
                    if let Err(err) = stream.write_all(&write_buf).await {
                        break Err(err.into());
                    }
                    write_buf.clear();
                }
            }
        }
    };

    let _ = pubsub_state.unsubscribe_all(&pubsub_hub);
    let _ = pubsub_state.punsubscribe_all(&pubsub_hub);
    pubsub_hub.cleanup_connection(pubsub_state.id);
    result
}

fn parse_next_frame(src: &mut BytesMut) -> Result<Option<RespFrame>, ParseError> {
    match parse_frame(src) {
        Ok(frame) => Ok(frame),
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

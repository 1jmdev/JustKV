use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;

use crate::pubsub::{ConnectionPubSub, PubSubHub};
use crate::transaction::TransactionState;
use engine::store::Store;
use protocol::encoder::encode;
use protocol::parser::{ParseError, parse_frame};
use protocol::types::{BulkData, RespFrame};

mod dispatch;
mod notifications;
mod util;

const READ_BUFFER_INITIAL: usize = 16 * 1024;
const WRITE_BUFFER_INITIAL: usize = 16 * 1024;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);
    let mut command_args_buf = Vec::with_capacity(16);
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

                while let Some(parsed) = parse_next_frame(&mut read_buf)? {
                    let command_name = command_name_from_frame(&parsed);
                    let _trace = command_name
                        .as_ref()
                        .and_then(|name| profiler::begin_request(name.as_bytes()));

                    let response = tx_state.handle_frame_with(&store, parsed, |store, frame| {
                        dispatch::execute_regular_command(
                            store,
                            &pubsub_hub,
                            &push_tx,
                            &mut pubsub_state,
                            &mut command_args_buf,
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
    let _trace = profiler::scope("server::connection::parse_next_frame");
    match parse_frame(src) {
        Ok(Some(frame)) => Ok(Some(frame)),
        Ok(None) | Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

fn command_name_from_frame(frame: &RespFrame) -> Option<CommandName> {
    let _trace = profiler::scope("server::connection::command_name_from_frame");
    let RespFrame::Array(Some(items)) = frame else {
        return None;
    };
    let src = match items.first()? {
        RespFrame::Bulk(Some(BulkData::Arg(arg))) => arg.as_slice(),
        RespFrame::Bulk(Some(BulkData::Value(value))) => value.as_slice(),
        RespFrame::Simple(value) => value.as_bytes(),
        RespFrame::SimpleStatic(value) => value.as_bytes(),
        _ => return None,
    };
    Some(CommandName::from_slice(src))
}

const CMD_NAME_MAX: usize = 32;

struct CommandName {
    len: u8,
    data: [u8; CMD_NAME_MAX],
}

impl CommandName {
    fn from_slice(src: &[u8]) -> Self {
        let len = src.len().min(CMD_NAME_MAX);
        let mut data = [0u8; CMD_NAME_MAX];
        data[..len].copy_from_slice(&src[..len]);
        data[..len].make_ascii_uppercase();
        Self { len: len as u8, data }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

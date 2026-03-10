use std::sync::Arc;

use bytes::BytesMut;
use commands::dispatch::identify;
use commands::transaction::TransactionState;
use engine::pubsub::{ConnectionPubSub, PubSubHub, PubSubMessage, PubSubSink, SharedPubSubSink};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::unbounded_channel;

use crate::auth::AuthService;
use crate::persistence::PersistenceHandle;
use crate::profile::ProfileHub;
use engine::store::Store;
use protocol::encoder::Encoder;
use protocol::parser::parse_command_into;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

mod dispatch;
mod notifications;
mod util;

struct ConnectionPushSink {
    push_tx: tokio::sync::mpsc::UnboundedSender<RespFrame>,
}

impl PubSubSink for ConnectionPushSink {
    fn push(&self, message: PubSubMessage) -> bool {
        self.push_tx.send(message_to_frame(message)).is_ok()
    }
}

fn message_to_frame(message: PubSubMessage) -> RespFrame {
    match message {
        PubSubMessage::Message { channel, payload } => RespFrame::Array(Some(vec![
            bulk_static(b"message"),
            RespFrame::Bulk(Some(BulkData::Arg(channel))),
            RespFrame::Bulk(Some(BulkData::Arg(payload))),
        ])),
        PubSubMessage::PatternMessage {
            pattern,
            channel,
            payload,
        } => RespFrame::Array(Some(vec![
            bulk_static(b"pmessage"),
            RespFrame::Bulk(Some(BulkData::Arg(pattern))),
            RespFrame::Bulk(Some(BulkData::Arg(channel))),
            RespFrame::Bulk(Some(BulkData::Arg(payload))),
        ])),
        PubSubMessage::ShardMessage { channel, payload } => RespFrame::Array(Some(vec![
            bulk_static(b"smessage"),
            RespFrame::Bulk(Some(BulkData::Arg(channel))),
            RespFrame::Bulk(Some(BulkData::Arg(payload))),
        ])),
    }
}

fn bulk_static(value: &'static [u8]) -> RespFrame {
    RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(value))))
}

const READ_BUFFER_INITIAL: usize = 16 * 1024;
const WRITE_BUFFER_INITIAL: usize = 16 * 1024;
const PUSH_DRAIN_BATCH: usize = 128;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
    auth: AuthService,
    persistence: PersistenceHandle,
    profiler: ProfileHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::connection::handle_connection");
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);

    let mut encoder = Encoder::default();
    let mut persistence_buf = Vec::with_capacity(1024);
    let mut persistence_dirty = 0u64;

    let mut command_args_buf = Vec::with_capacity(16);
    let mut tx_state = TransactionState::default();

    let (push_tx, mut push_rx) = unbounded_channel::<RespFrame>();
    let pubsub_sink: SharedPubSubSink = Arc::new(ConnectionPushSink {
        push_tx: push_tx.clone(),
    });
    let mut pubsub_state = ConnectionPubSub::new(pubsub_hub.next_connection_id());
    let mut client_state = dispatch::ClientState::default();
    let mut auth_state = auth.new_session();

    let result = loop {
        tokio::select! {
            push = push_rx.recv() => {
                let Some(frame) = push else {
                    break Ok(());
                };
                encoder.encode(&frame, &mut write_buf);

                let mut drained = 0;
                while drained < PUSH_DRAIN_BATCH {
                    match push_rx.try_recv() {
                        Ok(frame) => {
                            encoder.encode(&frame, &mut write_buf);
                            drained += 1;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => break,
                    }
                }

                if let Err(err) = flush_write_buf(&mut stream, &mut write_buf).await {
                    break Err(err.into());
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

                while parse_command_into(&mut read_buf, &mut command_args_buf)?.is_some() {
                    #[cfg(feature = "profiling")]
                    let _trace = if profiler.is_enabled() {
                        command_args_buf
                            .first()
                            .map(|command| profiler::begin_request_unconditional(command.as_slice()))
                    } else {
                        command_args_buf
                            .first()
                            .and_then(|command| profiler::begin_request(command.as_slice()))
                    };
                    #[cfg(not(feature = "profiling"))]
                    let _trace = command_args_buf
                        .first()
                        .and_then(|command| profiler::begin_request(command.as_slice()));

                    let command = identify(command_args_buf[0].as_slice());
                    let outcome = tx_state.handle_args_with(&store, &mut command_args_buf, command, |store, command, args| {
                        dispatch::execute_regular_command(
                            store,
                            &pubsub_hub,
                            &pubsub_sink,
                            &mut pubsub_state,
                            &mut client_state,
                            &auth,
                            &mut auth_state,
                            &profiler,
                            command,
                            args,
                        )
                    });
                    let response = outcome.response;

                    if !outcome.committed_commands.is_empty() {
                        persistence.record_transaction_to_buffer(
                            &outcome.committed_commands,
                            &mut persistence_buf,
                            &mut persistence_dirty,
                        );
                    } else {
                        persistence.record_command_to_buffer(
                            command,
                            &command_args_buf,
                            &response,
                            &mut persistence_buf,
                            &mut persistence_dirty,
                        );
                    }

                    if !client_state.take_suppress_current_reply() {
                        encoder.encode(&response, &mut write_buf);
                    }
                }

                persistence.flush_buffer(&mut persistence_buf, &mut persistence_dirty);
                if let Err(err) = flush_write_buf(&mut stream, &mut write_buf).await {
                    break Err(err.into());
                }
            }
        }
    };

    persistence.flush_buffer(&mut persistence_buf, &mut persistence_dirty);
    tx_state.cleanup(&store);
    pubsub_hub.cleanup_connection(pubsub_state.id());
    result
}

#[inline]
async fn flush_write_buf(stream: &mut TcpStream, write_buf: &mut BytesMut) -> std::io::Result<()> {
    if write_buf.is_empty() {
        return Ok(());
    }
    stream.write_all(write_buf).await?;
    write_buf.clear();
    Ok(())
}

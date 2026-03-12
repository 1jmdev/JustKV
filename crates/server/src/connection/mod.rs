use std::sync::Arc;

use bytes::{BytesMut, Buf};
use commands::dispatch::CommandId;
use commands::dispatch::identify;
use commands::transaction::TransactionState;
use engine::pubsub::{ConnectionPubSub, PubSubHub, PubSubMessage, PubSubSink, SharedPubSubSink};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::auth::AuthService;
use crate::persistence::PersistenceHandle;
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

#[derive(Default)]
struct PubSubSession {
    state: Option<ConnectionPubSub>,
    push_rx: Option<UnboundedReceiver<RespFrame>>,
    sink: Option<SharedPubSubSink>,
}

impl PubSubSession {
    fn subscription_count(&self) -> i64 {
        self.state
            .as_ref()
            .map_or(0, ConnectionPubSub::subscription_count)
    }

    fn ensure_active(&mut self, hub: &PubSubHub) -> (&mut ConnectionPubSub, &SharedPubSubSink) {
        if self.state.is_none() {
            let (push_tx, push_rx) = unbounded_channel::<RespFrame>();
            self.push_rx = Some(push_rx);
            self.sink = Some(Arc::new(ConnectionPushSink { push_tx }));
            self.state = Some(ConnectionPubSub::new(hub.next_connection_id()));
        }

        (
            self.state.as_mut().expect("pubsub state initialized"),
            self.sink.as_ref().expect("pubsub sink initialized"),
        )
    }

    fn subscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> i64 {
        let (state, sink) = self.ensure_active(hub);
        state.subscribe(hub, channel, sink);
        state.subscription_count()
    }

    fn unsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> i64 {
        let Some(state) = self.state.as_mut() else {
            return 0;
        };
        state.unsubscribe(hub, channel);
        state.subscription_count()
    }

    fn unsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let Some(state) = self.state.as_mut() else {
            return Vec::new();
        };
        state.unsubscribe_all(hub)
    }

    fn psubscribe(&mut self, hub: &PubSubHub, pattern: &[u8]) -> i64 {
        let (state, sink) = self.ensure_active(hub);
        state.psubscribe(hub, pattern, sink);
        state.subscription_count()
    }

    fn punsubscribe(&mut self, hub: &PubSubHub, pattern: &[u8]) -> i64 {
        let Some(state) = self.state.as_mut() else {
            return 0;
        };
        state.punsubscribe(hub, pattern);
        state.subscription_count()
    }

    fn punsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let Some(state) = self.state.as_mut() else {
            return Vec::new();
        };
        state.punsubscribe_all(hub)
    }

    fn ssubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> i64 {
        let (state, sink) = self.ensure_active(hub);
        state.ssubscribe(hub, channel, sink);
        state.subscription_count()
    }

    fn sunsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> i64 {
        let Some(state) = self.state.as_mut() else {
            return 0;
        };
        state.sunsubscribe(hub, channel);
        state.subscription_count()
    }

    fn sunsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let Some(state) = self.state.as_mut() else {
            return Vec::new();
        };
        state.sunsubscribe_all(hub)
    }

    fn cleanup(&self, hub: &PubSubHub) {
        if let Some(state) = self.state.as_ref() {
            hub.cleanup_connection(state.id());
        }
    }
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

const READ_BUFFER_INITIAL: usize = 64 * 1024;
const WRITE_BUFFER_INITIAL: usize = 64 * 1024;
const PUSH_DRAIN_BATCH: usize = 128;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
    auth: AuthService,
    persistence: PersistenceHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);

    let mut encoder = Encoder::default();
    let persistence_enabled = persistence.is_enabled();
    let mut persistence_buf = if persistence_enabled {
        Vec::with_capacity(1024)
    } else {
        Vec::new()
    };
    let mut persistence_dirty = 0u64;

    let mut command_args_buf = Vec::with_capacity(16);
    let mut tx_state = TransactionState::default();

    let mut pubsub = PubSubSession::default();
    let mut client_state = dispatch::ClientState::default();
    let mut auth_state = auth.new_session();

    let result = loop {
        if pubsub.push_rx.is_some() {
            tokio::select! {
                biased;
                read_result = read_into_buffer(&mut stream, &mut read_buf) => {
                    let bytes_read = match read_result {
                        Ok(value) => value,
                        Err(err) => break Err(err.into()),
                    };
                    if bytes_read == 0 {
                        break Ok(());
                    }

                    if let Err(err) = process_read_buf(
                        &mut read_buf,
                        &store,
                        &pubsub_hub,
                        &auth,
                        &persistence,
                        persistence_enabled,
                        &mut encoder,
                        &mut write_buf,
                        &mut persistence_buf,
                        &mut persistence_dirty,
                        &mut command_args_buf,
                        &mut tx_state,
                        &mut pubsub,
                        &mut client_state,
                        &mut auth_state,
                    ) {
                        break Err(err.into());
                    }

                    if let Err(err) = flush_write_buf(&mut stream, &mut write_buf).await {
                        break Err(err.into());
                    }
                }
                push = pubsub.push_rx.as_mut().expect("push receiver active").recv() => {
                    let Some(frame) = push else {
                        break Ok(());
                    };
                    encoder.encode(&frame, &mut write_buf);

                    let push_rx = pubsub.push_rx.as_mut().expect("push receiver active");
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
            }
        } else {
            let bytes_read = match read_into_buffer(&mut stream, &mut read_buf).await {
                Ok(value) => value,
                Err(err) => break Err(err.into()),
            };
            if bytes_read == 0 {
                break Ok(());
            }

            if let Err(err) = process_read_buf(
                &mut read_buf,
                &store,
                &pubsub_hub,
                &auth,
                &persistence,
                persistence_enabled,
                &mut encoder,
                &mut write_buf,
                &mut persistence_buf,
                &mut persistence_dirty,
                &mut command_args_buf,
                &mut tx_state,
                &mut pubsub,
                &mut client_state,
                &mut auth_state,
            ) {
                break Err(err.into());
            }

            if let Err(err) = flush_write_buf(&mut stream, &mut write_buf).await {
                break Err(err.into());
            }
        }
    };

    if persistence_enabled {
        persistence.flush_buffer(&mut persistence_buf, &mut persistence_dirty);
    }
    tx_state.cleanup(&store);
    pubsub.cleanup(&pubsub_hub);
    result
}

#[allow(clippy::too_many_arguments)]
fn process_read_buf(
    read_buf: &mut BytesMut,
    store: &Store,
    pubsub_hub: &PubSubHub,
    auth: &AuthService,
    persistence: &PersistenceHandle,
    persistence_enabled: bool,
    encoder: &mut Encoder,
    write_buf: &mut BytesMut,
    persistence_buf: &mut Vec<u8>,
    persistence_dirty: &mut u64,
    command_args_buf: &mut Vec<CompactArg>,
    tx_state: &mut TransactionState,
    pubsub: &mut PubSubSession,
    client_state: &mut dispatch::ClientState,
    auth_state: &mut crate::auth::SessionAuth,
) -> Result<(), protocol::parser::ParseError> {
    while parse_command_into(read_buf, command_args_buf)?.is_some() {
        let _trace = command_args_buf
            .first();
        let command = identify(command_args_buf[0].as_slice());
        let response = if tx_state.is_plain_mode() && !is_transaction_command(command) {
            store.with_command_gate(|| {
                dispatch::execute_regular_command(
                    store,
                    pubsub_hub,
                    pubsub,
                    client_state,
                    auth,
                    auth_state,
                    command,
                    command_args_buf,
                )
            })
        } else {
            let outcome = tx_state.handle_args_with(
                store,
                command_args_buf,
                command,
                |store, command, args| {
                    dispatch::execute_regular_command(
                        store,
                        pubsub_hub,
                        pubsub,
                        client_state,
                        auth,
                        auth_state,
                        command,
                        args,
                    )
                },
            );

            if persistence_enabled {
                if !outcome.committed_commands.is_empty() {
                    persistence.record_transaction_to_buffer(
                        &outcome.committed_commands,
                        persistence_buf,
                        persistence_dirty,
                    );
                } else {
                    persistence.record_command_to_buffer(
                        command,
                        command_args_buf,
                        &outcome.response,
                        persistence_buf,
                        persistence_dirty,
                    );
                }
            }

            outcome.response
        };

        if persistence_enabled && tx_state.is_plain_mode() && !is_transaction_command(command) {
            persistence.record_command_to_buffer(
                command,
                command_args_buf,
                &response,
                persistence_buf,
                persistence_dirty,
            );
        }

        if !client_state.take_suppress_current_reply() {
            encoder.encode(&response, write_buf);
        }
    }

    if persistence_enabled {
        persistence.flush_buffer(persistence_buf, persistence_dirty);
    }
    Ok(())
}

#[inline(always)]
fn is_transaction_command(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::Multi
            | CommandId::Exec
            | CommandId::Discard
            | CommandId::Watch
            | CommandId::Unwatch
    )
}

async fn read_into_buffer(
    stream: &mut TcpStream,
    read_buf: &mut BytesMut,
) -> std::io::Result<usize> {
    let mut total_read = stream.read_buf(read_buf).await?;
    if total_read == 0 {
        return Ok(0);
    }

    loop {
        match stream.try_read_buf(read_buf) {
            Ok(0) => return Ok(total_read),
            Ok(read) => total_read += read,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => return Ok(total_read),
            Err(err) => return Err(err),
        }
    }
}

#[inline]
async fn flush_write_buf(
    stream: &mut TcpStream,
    write_buf: &mut BytesMut,
) -> std::io::Result<()> {
    if write_buf.is_empty() {
        return Ok(());
    }

    loop {
        match stream.try_write(write_buf.chunk()) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write returned zero",
                ));
            }
            Ok(n) => {
                write_buf.advance(n);
                if write_buf.is_empty() {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                stream.writable().await?;
            }
            Err(e) => return Err(e),
        }
    }
}

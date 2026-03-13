use std::sync::Arc;

use bytes::{Buf, BytesMut};
use commands::dispatch::CommandId;
use commands::dispatch::identify;
use commands::transaction::TransactionState;
use engine::pubsub::{ConnectionPubSub, PubSubHub, PubSubMessage, PubSubSink, SharedPubSubSink};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, UnixStream};
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

    async fn push_rx_recv(&mut self) -> Option<RespFrame> {
        match self.push_rx.as_mut() {
            Some(push_rx) => push_rx.recv().await,
            None => None,
        }
    }

    fn try_recv_push(&mut self) -> Result<RespFrame, TryRecvError> {
        match self.push_rx.as_mut() {
            Some(push_rx) => push_rx.try_recv(),
            None => Err(TryRecvError::Disconnected),
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

pub enum ConnectionStream {
    Tcp(TcpStream),
    Unix(UnixStream),
}

impl ConnectionStream {
    async fn read_buf(&mut self, buf: &mut BytesMut) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read_buf(buf).await,
            Self::Unix(stream) => stream.read_buf(buf).await,
        }
    }

    fn try_read_buf(&mut self, buf: &mut BytesMut) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.try_read_buf(buf),
            Self::Unix(stream) => stream.try_read_buf(buf),
        }
    }

    fn try_write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.try_write(buf),
            Self::Unix(stream) => stream.try_write(buf),
        }
    }

    async fn writable(&self) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.writable().await,
            Self::Unix(stream) => stream.writable().await,
        }
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub struct ConnectionShared {
    store: Store,
    pubsub_hub: PubSubHub,
    auth: AuthService,
    persistence: PersistenceHandle,
}

impl ConnectionShared {
    #[doc(hidden)]
    pub fn new(
        store: Store,
        pubsub_hub: PubSubHub,
        auth: AuthService,
        persistence: PersistenceHandle,
    ) -> Self {
        Self {
            store,
            pubsub_hub,
            auth,
            persistence,
        }
    }
}

struct ConnectionProcessor {
    shared: ConnectionShared,
    read_buf: BytesMut,
    write_buf: BytesMut,
    encoder: Encoder,
    persistence_enabled: bool,
    persistence_buf: Vec<u8>,
    persistence_dirty: u64,
    command_args_buf: Vec<CompactArg>,
    tx_state: TransactionState,
    pubsub: PubSubSession,
    client_state: dispatch::ClientState,
    auth_state: crate::auth::SessionAuth,
}

impl ConnectionProcessor {
    fn new(shared: ConnectionShared) -> Self {
        let persistence_enabled = shared.persistence.is_enabled();
        let persistence_buf = if persistence_enabled {
            Vec::with_capacity(1024)
        } else {
            Vec::new()
        };

        Self {
            auth_state: shared.auth.new_session(),
            shared,
            read_buf: BytesMut::with_capacity(READ_BUFFER_INITIAL),
            write_buf: BytesMut::with_capacity(WRITE_BUFFER_INITIAL),
            encoder: Encoder::default(),
            persistence_enabled,
            persistence_buf,
            persistence_dirty: 0,
            command_args_buf: Vec::with_capacity(16),
            tx_state: TransactionState::default(),
            pubsub: PubSubSession::default(),
            client_state: dispatch::ClientState::default(),
        }
    }

    async fn run(
        &mut self,
        stream: &mut ConnectionStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let result = loop {
            if self.pubsub.push_rx.is_some() {
                tokio::select! {
                    biased;
                    read_result = read_into_buffer(stream, &mut self.read_buf) => {
                        let bytes_read = match read_result {
                            Ok(value) => value,
                            Err(err) => break Err(err.into()),
                        };
                        if bytes_read == 0 {
                            break Ok(());
                        }

                        if let Err(err) = self.process_read_buf() {
                            break Err(err.into());
                        }

                        if let Err(err) = flush_write_buf(stream, &mut self.write_buf).await {
                            break Err(err.into());
                        }
                    }
                    push = self.pubsub.push_rx_recv() => {
                        let Some(frame) = push else {
                            break Ok(());
                        };
                        self.encoder.encode(&frame, &mut self.write_buf);

                        let mut drained = 0;
                        while drained < PUSH_DRAIN_BATCH {
                            match self.pubsub.try_recv_push() {
                                Ok(frame) => {
                                    self.encoder.encode(&frame, &mut self.write_buf);
                                    drained += 1;
                                }
                                Err(TryRecvError::Empty) => break,
                                Err(TryRecvError::Disconnected) => break,
                            }
                        }

                        if let Err(err) = flush_write_buf(stream, &mut self.write_buf).await {
                            break Err(err.into());
                        }
                    }
                }
            } else {
                let bytes_read = match read_into_buffer(stream, &mut self.read_buf).await {
                    Ok(value) => value,
                    Err(err) => break Err(err.into()),
                };
                if bytes_read == 0 {
                    break Ok(());
                }

                if let Err(err) = self.process_read_buf() {
                    break Err(err.into());
                }

                if let Err(err) = flush_write_buf(stream, &mut self.write_buf).await {
                    break Err(err.into());
                }
            }
        };

        if self.persistence_enabled {
            self.shared
                .persistence
                .flush_buffer(&mut self.persistence_buf, &mut self.persistence_dirty);
        }
        self.tx_state.cleanup(&self.shared.store);
        self.pubsub.cleanup(&self.shared.pubsub_hub);
        result
    }

    fn process_read_buf(&mut self) -> Result<(), protocol::parser::ParseError> {
        while parse_command_into(&mut self.read_buf, &mut self.command_args_buf)?.is_some() {
            let _trace = self.command_args_buf.first();
            let command = identify(self.command_args_buf[0].as_slice());
            let response = if self.tx_state.is_plain_mode() && !is_transaction_command(command) {
                self.shared.store.with_command_gate(|| {
                    dispatch::execute_regular_command(
                        &self.shared,
                        &mut self.pubsub,
                        &mut self.client_state,
                        &mut self.auth_state,
                        command,
                        &self.command_args_buf,
                    )
                })
            } else {
                let shared = &self.shared;
                let pubsub = &mut self.pubsub;
                let client_state = &mut self.client_state;
                let auth_state = &mut self.auth_state;
                let outcome = self.tx_state.handle_args_with(
                    &shared.store,
                    &mut self.command_args_buf,
                    command,
                    |_store, command, args| {
                        dispatch::execute_regular_command(
                            shared,
                            pubsub,
                            client_state,
                            auth_state,
                            command,
                            args,
                        )
                    },
                );

                if self.persistence_enabled {
                    if !outcome.committed_commands.is_empty() {
                        self.shared.persistence.record_transaction_to_buffer(
                            &outcome.committed_commands,
                            &mut self.persistence_buf,
                            &mut self.persistence_dirty,
                        );
                    } else {
                        self.shared.persistence.record_command_to_buffer(
                            command,
                            &self.command_args_buf,
                            &outcome.response,
                            &mut self.persistence_buf,
                            &mut self.persistence_dirty,
                        );
                    }
                }

                outcome.response
            };

            if self.persistence_enabled
                && self.tx_state.is_plain_mode()
                && !is_transaction_command(command)
            {
                self.shared.persistence.record_command_to_buffer(
                    command,
                    &self.command_args_buf,
                    &response,
                    &mut self.persistence_buf,
                    &mut self.persistence_dirty,
                );
            }

            if !self.client_state.take_suppress_current_reply() {
                self.encoder.encode(&response, &mut self.write_buf);
            }
        }

        if self.persistence_enabled {
            self.shared
                .persistence
                .flush_buffer(&mut self.persistence_buf, &mut self.persistence_dirty);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub async fn handle_connection(
    mut stream: ConnectionStream,
    shared: ConnectionShared,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ConnectionProcessor::new(shared).run(&mut stream).await
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
    stream: &mut ConnectionStream,
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
    stream: &mut ConnectionStream,
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

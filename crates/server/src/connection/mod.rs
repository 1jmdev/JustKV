use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::unbounded_channel;

use crate::auth::AuthService;
use crate::profile::ProfileHub;
use crate::pubsub::{ConnectionPubSub, PubSubHub};
use crate::transaction::TransactionState;
use engine::store::Store;
use protocol::encoder::Encoder;
use protocol::parser::parse_command_into;
use protocol::types::RespFrame;

mod dispatch;
mod notifications;
mod util;

const READ_BUFFER_INITIAL: usize = 16 * 1024;
const WRITE_BUFFER_INITIAL: usize = 16 * 1024;
const PUSH_DRAIN_BATCH: usize = 128;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
    auth: AuthService,
    profiler: ProfileHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::connection::handle_connection");
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);

    let mut encoder = Encoder::default();

    let mut command_args_buf = Vec::with_capacity(16);
    let mut tx_state = TransactionState::default();

    let (push_tx, mut push_rx) = unbounded_channel::<RespFrame>();
    let mut pubsub_state = ConnectionPubSub::new(pubsub_hub.next_connection_id());
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

                    let response = tx_state.handle_args_with(&store, &mut command_args_buf, |store, args| {
                        dispatch::execute_regular_command(
                            store,
                            &pubsub_hub,
                            &push_tx,
                            &mut pubsub_state,
                            &auth,
                            &mut auth_state,
                            &profiler,
                            args,
                        )
                    });

                    encoder.encode(&response, &mut write_buf);
                }

                if let Err(err) = flush_write_buf(&mut stream, &mut write_buf).await {
                    break Err(err.into());
                }
            }
        }
    };

    let _ = pubsub_state.unsubscribe_all(&pubsub_hub);
    let _ = pubsub_state.punsubscribe_all(&pubsub_hub);
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

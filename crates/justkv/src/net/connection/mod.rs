use bytes::BytesMut;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::block_in_place;

use crate::engine::store::Store;
use crate::net::profiling::LatencyProfiler;
use crate::net::pubsub::{ConnectionPubSub, PubSubHub};
use crate::net::transaction::TransactionState;
use crate::protocol::encoder::encode;
use crate::protocol::parser::{ParseError, parse_frame};
use crate::protocol::types::{BulkData, RespFrame};

mod dispatch;
mod notifications;
mod util;

const READ_BUFFER_INITIAL: usize = 16 * 1024;
const WRITE_BUFFER_INITIAL: usize = 16 * 1024;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
    profiler: Option<Arc<LatencyProfiler>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);
    let mut command_args_buf = Vec::with_capacity(16);
    let mut tx_state = TransactionState::default();
    let profiling_enabled = profiler.is_some();

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
                    let write_started = Instant::now();
                    if let Err(err) = stream.write_all(&write_buf).await {
                        break Err(err.into());
                    }
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_write(write_started.elapsed());
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

                while let Some(parsed) = parse_next_frame(&mut read_buf, profiling_enabled)? {
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_parse(parsed.parse_elapsed);
                    }
                    let command_name = if profiling_enabled {
                        command_name_from_frame(&parsed.frame)
                    } else {
                        None
                    };
                    let execute_started = profiling_enabled.then(Instant::now);
                    let response = block_in_place(|| {
                        tx_state.handle_frame_with(&store, parsed.frame, |store, frame| {
                            dispatch::execute_regular_command(
                                store,
                                &pubsub_hub,
                                &push_tx,
                                &mut pubsub_state,
                                &mut command_args_buf,
                                profiler.as_ref(),
                                frame,
                            )
                        })
                    });
                    let execute_elapsed = execute_started
                        .map(|started| started.elapsed())
                        .unwrap_or_default();
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_execute(execute_elapsed);
                    }
                    let encode_started = profiling_enabled.then(Instant::now);
                    encode(&response, &mut write_buf);
                    let encode_elapsed = encode_started
                        .map(|started| started.elapsed())
                        .unwrap_or_default();
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_encode(encode_elapsed);
                        if let Some(ref command_name) = command_name {
                            profiler.record_request(
                                command_name.as_bytes(),
                                parsed.parse_elapsed,
                                execute_elapsed,
                                encode_elapsed,
                            );
                        }
                    }
                }

                if !write_buf.is_empty() {
                    let write_started = Instant::now();
                    if let Err(err) = stream.write_all(&write_buf).await {
                        break Err(err.into());
                    }
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_write(write_started.elapsed());
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

struct ParsedFrame {
    frame: RespFrame,
    parse_elapsed: std::time::Duration,
}

fn parse_next_frame(
    src: &mut BytesMut,
    measure_latency: bool,
) -> Result<Option<ParsedFrame>, ParseError> {
    let parse_started = measure_latency.then(Instant::now);
    match parse_frame(src) {
        Ok(Some(frame)) => Ok(Some(ParsedFrame {
            frame,
            parse_elapsed: parse_started
                .map(|started| started.elapsed())
                .unwrap_or_default(),
        })),
        Ok(None) => Ok(None),
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Extract and uppercase-normalize the command name from a parsed RESP frame.
/// Returns a stack-allocated array to avoid a heap allocation on every request.
fn command_name_from_frame(frame: &RespFrame) -> Option<CommandName> {
    let RespFrame::Array(Some(items)) = frame else {
        return None;
    };
    let command = items.first()?;

    let src = match command {
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
        Self {
            len: len as u8,
            data,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

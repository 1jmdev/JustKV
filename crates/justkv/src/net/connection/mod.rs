use bytes::BytesMut;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;

use crate::engine::store::Store;
use crate::net::profiling::LatencyProfiler;
use crate::net::pubsub::{ConnectionPubSub, PubSubHub};
use crate::net::transaction::TransactionState;
use crate::protocol::encoder::encode;
use crate::protocol::parser::{parse_frame, ParseError};
use crate::protocol::types::{BulkData, RespFrame};

mod dispatch;
mod notifications;
mod util;

const READ_BUFFER_INITIAL: usize = 4 * 1024;
const WRITE_BUFFER_INITIAL: usize = 4 * 1024;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Store,
    pubsub_hub: PubSubHub,
    profiler: Option<Arc<LatencyProfiler>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = BytesMut::with_capacity(READ_BUFFER_INITIAL);
    let mut write_buf = BytesMut::with_capacity(WRITE_BUFFER_INITIAL);
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

                while let Some(parsed) = parse_next_frame(&mut read_buf)? {
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_parse(parsed.parse_elapsed);
                    }
                    let command_name = command_name_from_frame(&parsed.frame);
                    let execute_started = Instant::now();
                    let response = tx_state.handle_frame_with(&store, parsed.frame, |store, frame| {
                        dispatch::execute_regular_command(
                            store,
                            &pubsub_hub,
                            &push_tx,
                            &mut pubsub_state,
                            profiler.as_ref(),
                            frame,
                        )
                    });
                    let execute_elapsed = execute_started.elapsed();
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_execute(execute_elapsed);
                    }
                    let encode_started = Instant::now();
                    encode(&response, &mut write_buf);
                    let encode_elapsed = encode_started.elapsed();
                    if let Some(profiler) = profiler.as_ref() {
                        profiler.record_encode(encode_elapsed);
                        if let Some(command_name) = command_name.as_deref() {
                            profiler.record_request(
                                command_name,
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

fn parse_next_frame(src: &mut BytesMut) -> Result<Option<ParsedFrame>, ParseError> {
    let parse_started = Instant::now();
    match parse_frame(src) {
        Ok(Some(frame)) => Ok(Some(ParsedFrame {
            frame,
            parse_elapsed: parse_started.elapsed(),
        })),
        Ok(None) => Ok(None),
        Err(ParseError::Incomplete) => Ok(None),
        Err(err) => Err(err),
    }
}

fn command_name_from_frame(frame: &RespFrame) -> Option<Vec<u8>> {
    let RespFrame::Array(Some(items)) = frame else {
        return None;
    };
    let command = items.first()?;

    let mut out = match command {
        RespFrame::Bulk(Some(BulkData::Arg(arg))) => arg.as_slice().to_vec(),
        RespFrame::Bulk(Some(BulkData::Value(value))) => value.as_slice().to_vec(),
        RespFrame::Simple(value) => value.as_bytes().to_vec(),
        RespFrame::SimpleStatic(value) => value.as_bytes().to_vec(),
        _ => return None,
    };
    out.make_ascii_uppercase();
    Some(out)
}

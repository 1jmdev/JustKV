use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc::UnboundedSender;

use crate::commands::dispatcher::{dispatch_args, parse_command};
use crate::engine::store::Store;
use crate::engine::value::CompactArg;
use crate::net::profiling::LatencyProfiler;
use crate::protocol::types::{BulkData, RespFrame};

use super::super::pubsub::{ConnectionPubSub, PubSubHub};
use super::notifications::emit_command_notifications;
use super::util::{collapse_pubsub_responses, wrong_args};

pub(super) fn execute_regular_command(
    store: &Store,
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    profiler: Option<&Arc<LatencyProfiler>>,
    frame: RespFrame,
) -> RespFrame {
    let started = Instant::now();
    let args = match parse_command(frame) {
        Ok(value) => value,
        Err(err) => return RespFrame::error_static(err),
    };
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    if let Some(response) = handle_pubsub_or_config_command(hub, push_tx, pubsub_state, &args) {
        return response;
    }

    let command = args[0].as_slice();
    let response = dispatch_args(store, &args);
    emit_command_notifications(hub, command, &args, &response);
    if let Some(profiler) = profiler {
        profiler.record_command(command, started.elapsed());
    }
    response
}

fn handle_pubsub_or_config_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> Option<RespFrame> {
    let command = args[0].as_slice();

    if command == b"PUBLISH" {
        return Some(publish_command(hub, args));
    }
    if command == b"SUBSCRIBE" {
        return Some(subscribe_command(hub, push_tx, pubsub_state, args));
    }
    if command == b"UNSUBSCRIBE" {
        return Some(unsubscribe_command(hub, pubsub_state, args));
    }
    if command == b"PSUBSCRIBE" {
        return Some(psubscribe_command(hub, push_tx, pubsub_state, args));
    }
    if command == b"PUNSUBSCRIBE" {
        return Some(punsubscribe_command(hub, pubsub_state, args));
    }
    if command == b"PUBSUB" {
        return Some(pubsub_command(hub, args));
    }
    if command == b"CONFIG" {
        return Some(config_command(hub, args));
    }
    None
}

fn publish_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("PUBLISH");
    }
    RespFrame::Integer(hub.publish(&args[1], &args[2]))
}

fn subscribe_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for channel in &args[1..] {
        pubsub_state.subscribe(hub, channel, push_tx);
        responses.push(RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"subscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::Arg(channel.clone()))),
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn unsubscribe_command(
    hub: &PubSubHub,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let channels = if args.len() == 1 {
        let existing = pubsub_state.unsubscribe_all(hub);
        if existing.is_empty() {
            vec![CompactArg::from_vec(Vec::new())]
        } else {
            existing.into_iter().map(CompactArg::from_vec).collect()
        }
    } else {
        let mut out = Vec::with_capacity(args.len() - 1);
        for channel in &args[1..] {
            let _ = pubsub_state.unsubscribe(hub, channel);
            out.push(channel.clone());
        }
        out
    };

    let mut responses = Vec::with_capacity(channels.len());
    for channel in channels {
        responses.push(RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"unsubscribe".to_vec()))),
            if channel.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Bulk(Some(BulkData::Arg(channel)))
            },
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }

    collapse_pubsub_responses(responses)
}

fn psubscribe_command(
    hub: &PubSubHub,
    push_tx: &UnboundedSender<RespFrame>,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("PSUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for pattern in &args[1..] {
        pubsub_state.psubscribe(hub, pattern, push_tx);
        responses.push(RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"psubscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::Arg(pattern.clone()))),
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }
    collapse_pubsub_responses(responses)
}

fn punsubscribe_command(
    hub: &PubSubHub,
    pubsub_state: &mut ConnectionPubSub,
    args: &[CompactArg],
) -> RespFrame {
    let patterns = if args.len() == 1 {
        let existing = pubsub_state.punsubscribe_all(hub);
        if existing.is_empty() {
            vec![CompactArg::from_vec(Vec::new())]
        } else {
            existing.into_iter().map(CompactArg::from_vec).collect()
        }
    } else {
        let mut out = Vec::with_capacity(args.len() - 1);
        for pattern in &args[1..] {
            let _ = pubsub_state.punsubscribe(hub, pattern);
            out.push(pattern.clone());
        }
        out
    };

    let mut responses = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        responses.push(RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"punsubscribe".to_vec()))),
            if pattern.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Bulk(Some(BulkData::Arg(pattern)))
            },
            RespFrame::Integer(pubsub_state.subscription_count()),
        ])));
    }

    collapse_pubsub_responses(responses)
}

fn pubsub_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("PUBSUB");
    }

    let subcommand = args[1].as_slice();
    if subcommand.eq_ignore_ascii_case(b"CHANNELS") {
        let pattern = if args.len() == 3 {
            Some(args[2].as_slice())
        } else if args.len() == 2 {
            None
        } else {
            return wrong_args("PUBSUB");
        };
        let channels = hub.pubsub_channels(pattern);
        return RespFrame::Array(Some(
            channels
                .into_iter()
                .map(|channel| RespFrame::Bulk(Some(BulkData::from_vec(channel))))
                .collect(),
        ));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMSUB") {
        let channels = args[2..]
            .iter()
            .map(|channel| channel.to_vec())
            .collect::<Vec<_>>();
        let counts = hub.pubsub_numsub(&channels);
        let mut response = Vec::with_capacity(counts.len() * 2);
        for (channel, count) in counts {
            response.push(RespFrame::Bulk(Some(BulkData::from_vec(channel))));
            response.push(RespFrame::Integer(count));
        }
        return RespFrame::Array(Some(response));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMPAT") {
        if args.len() != 2 {
            return wrong_args("PUBSUB");
        }
        return RespFrame::Integer(hub.pubsub_numpat());
    }

    RespFrame::Error("ERR Unknown PUBSUB subcommand".to_string())
}

fn config_command(hub: &PubSubHub, args: &[CompactArg]) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("CONFIG");
    }

    let subcommand = args[1].as_slice();
    if subcommand.eq_ignore_ascii_case(b"GET") {
        if args.len() != 3 {
            return wrong_args("CONFIG");
        }
        if !args[2]
            .as_slice()
            .eq_ignore_ascii_case(b"notify-keyspace-events")
        {
            return RespFrame::Array(Some(vec![]));
        }
        return RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"notify-keyspace-events".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(hub.get_notify_flags()))),
        ]));
    }

    if subcommand.eq_ignore_ascii_case(b"SET") {
        if args.len() != 4 {
            return wrong_args("CONFIG");
        }
        if !args[2]
            .as_slice()
            .eq_ignore_ascii_case(b"notify-keyspace-events")
        {
            return RespFrame::Error("ERR Unsupported CONFIG parameter".to_string());
        }
        return match hub.set_notify_flags(&args[3]) {
            Ok(()) => RespFrame::ok(),
            Err(()) => {
                RespFrame::Error("ERR CONFIG SET failed (possibly related to argument)".to_string())
            }
        };
    }

    RespFrame::Error("ERR Unknown subcommand or wrong number of arguments for CONFIG".to_string())
}

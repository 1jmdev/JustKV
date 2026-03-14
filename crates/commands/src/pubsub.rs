use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

use crate::dispatch::CommandId;
use crate::util::{wrong_args, Args};

pub trait DispatchContext {
    fn publish(&mut self, channel: &[u8], payload: &[u8]) -> i64;
    fn spublish(&mut self, channel: &[u8], payload: &[u8]) -> i64;
    fn subscribe(&mut self, channel: &[u8]) -> i64;
    fn unsubscribe(&mut self, channel: &[u8]) -> i64;
    fn unsubscribe_all(&mut self) -> Vec<Vec<u8>>;
    fn psubscribe(&mut self, pattern: &[u8]) -> i64;
    fn punsubscribe(&mut self, pattern: &[u8]) -> i64;
    fn punsubscribe_all(&mut self) -> Vec<Vec<u8>>;
    fn ssubscribe(&mut self, channel: &[u8]) -> i64;
    fn sunsubscribe(&mut self, channel: &[u8]) -> i64;
    fn sunsubscribe_all(&mut self) -> Vec<Vec<u8>>;
    fn subscription_count(&self) -> i64;
    fn pubsub_channels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>>;
    fn pubsub_numsub(&self, channels: &[CompactArg]) -> Vec<(Vec<u8>, i64)>;
    fn pubsub_numpat(&self) -> i64;
    fn pubsub_shardchannels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>>;
    fn pubsub_shardnumsub(&self, channels: &[CompactArg]) -> Vec<(Vec<u8>, i64)>;
    fn set_notify_flags(&mut self, flags: &[u8]) -> Result<(), ()>;
    fn get_notify_flags(&self) -> Vec<u8>;
}

pub fn dispatch(context: &mut dyn DispatchContext, command: CommandId, args: &Args) -> RespFrame {
    match command {
        CommandId::Publish => publish_command(context, args),
        CommandId::SPublish => spublish_command(context, args),
        CommandId::Subscribe => subscribe_command(context, args),
        CommandId::Unsubscribe => unsubscribe_command(context, args),
        CommandId::PSubscribe => psubscribe_command(context, args),
        CommandId::PUnsubscribe => punsubscribe_command(context, args),
        CommandId::SSubscribe => ssubscribe_command(context, args),
        CommandId::SUnsubscribe => sunsubscribe_command(context, args),
        CommandId::PubSub => pubsub_command(context, args),
        CommandId::Config => config_command(context, args),
        _ => RespFrame::error_static("ERR unknown command"),
    }
}

fn publish_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("PUBLISH");
    }
    RespFrame::Integer(context.publish(args[1].as_slice(), args[2].as_slice()))
}

fn spublish_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("SPUBLISH");
    }
    RespFrame::Integer(context.spublish(args[1].as_slice(), args[2].as_slice()))
}

fn subscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for channel in &args[1..] {
        let count = context.subscribe(channel.as_slice());
        responses.push(subscription_response(
            b"subscribe",
            channel.as_slice(),
            count,
        ));
    }
    collapse_pubsub_responses(responses)
}

fn unsubscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    let channels = if args.len() == 1 {
        context.unsubscribe_all()
    } else {
        let mut responses = Vec::with_capacity(args.len() - 1);
        for channel in &args[1..] {
            let count = context.unsubscribe(channel.as_slice());
            responses.push(unsubscribe_response(
                b"unsubscribe",
                Some(channel.as_slice()),
                count,
            ));
        }
        return collapse_pubsub_responses(responses);
    };

    unsubscribe_all_response(context.subscription_count(), channels, b"unsubscribe")
}

fn psubscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("PSUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for pattern in &args[1..] {
        let count = context.psubscribe(pattern.as_slice());
        responses.push(subscription_response(
            b"psubscribe",
            pattern.as_slice(),
            count,
        ));
    }
    collapse_pubsub_responses(responses)
}

fn punsubscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    let patterns = if args.len() == 1 {
        context.punsubscribe_all()
    } else {
        let mut responses = Vec::with_capacity(args.len() - 1);
        for pattern in &args[1..] {
            let count = context.punsubscribe(pattern.as_slice());
            responses.push(unsubscribe_response(
                b"punsubscribe",
                Some(pattern.as_slice()),
                count,
            ));
        }
        return collapse_pubsub_responses(responses);
    };

    unsubscribe_all_response(context.subscription_count(), patterns, b"punsubscribe")
}

fn ssubscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SSUBSCRIBE");
    }

    let mut responses = Vec::with_capacity(args.len() - 1);
    for channel in &args[1..] {
        let count = context.ssubscribe(channel.as_slice());
        responses.push(subscription_response(
            b"ssubscribe",
            channel.as_slice(),
            count,
        ));
    }
    collapse_pubsub_responses(responses)
}

fn sunsubscribe_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    let channels = if args.len() == 1 {
        context.sunsubscribe_all()
    } else {
        let mut responses = Vec::with_capacity(args.len() - 1);
        for channel in &args[1..] {
            let count = context.sunsubscribe(channel.as_slice());
            responses.push(unsubscribe_response(
                b"sunsubscribe",
                Some(channel.as_slice()),
                count,
            ));
        }
        return collapse_pubsub_responses(responses);
    };

    unsubscribe_all_response(context.subscription_count(), channels, b"sunsubscribe")
}

fn pubsub_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
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
        return array_of_bulk(context.pubsub_channels(pattern));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMSUB") {
        return paired_counts_response(context.pubsub_numsub(&args[2..]));
    }

    if subcommand.eq_ignore_ascii_case(b"NUMPAT") {
        if args.len() != 2 {
            return wrong_args("PUBSUB");
        }
        return RespFrame::Integer(context.pubsub_numpat());
    }

    if subcommand.eq_ignore_ascii_case(b"SHARDCHANNELS") {
        let pattern = if args.len() == 3 {
            Some(args[2].as_slice())
        } else if args.len() == 2 {
            None
        } else {
            return wrong_args("PUBSUB");
        };
        return array_of_bulk(context.pubsub_shardchannels(pattern));
    }

    if subcommand.eq_ignore_ascii_case(b"SHARDNUMSUB") {
        return paired_counts_response(context.pubsub_shardnumsub(&args[2..]));
    }

    unknown_subcommand_error(subcommand)
}

fn config_command(context: &mut dyn DispatchContext, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("CONFIG");
    }

    let subcommand = args[1].as_slice();
    if subcommand.eq_ignore_ascii_case(b"GET") {
        if args.len() < 3 {
            return wrong_args("CONFIG");
        }

        let mut response = Vec::new();
        for pattern in &args[2..] {
            append_config_matches(context, pattern.as_slice(), &mut response);
        }
        return RespFrame::Array(Some(response));
    }

    if subcommand.eq_ignore_ascii_case(b"SET") {
        if args.len() != 4 {
            return wrong_args("CONFIG");
        }
        if args[2]
            .as_slice()
            .eq_ignore_ascii_case(b"notify-keyspace-events")
        {
            return match context.set_notify_flags(args[3].as_slice()) {
                Ok(()) => RespFrame::ok(),
                Err(()) => {
                    RespFrame::error_static("ERR CONFIG SET failed (possibly related to argument)")
                }
            };
        }
        return RespFrame::ok();
    }

    if subcommand.eq_ignore_ascii_case(b"RESETSTAT") || subcommand.eq_ignore_ascii_case(b"REWRITE")
    {
        if args.len() != 2 {
            return wrong_args("CONFIG");
        }
        return RespFrame::ok();
    }

    unknown_subcommand_error(subcommand)
}

fn append_config_matches(context: &dyn DispatchContext, pattern: &[u8], out: &mut Vec<RespFrame>) {
    if config_match(pattern, b"notify-keyspace-events") {
        out.push(bulk_static(b"notify-keyspace-events"));
        out.push(RespFrame::Bulk(Some(BulkData::from_vec(
            context.get_notify_flags(),
        ))));
    }
    if config_match(pattern, b"maxmemory") {
        out.push(bulk_static(b"maxmemory"));
        out.push(bulk_static(b"0"));
    }
    if config_match(pattern, b"timeout") {
        out.push(bulk_static(b"timeout"));
        out.push(bulk_static(b"0"));
    }
}

fn config_match(pattern: &[u8], name: &[u8]) -> bool {
    if pattern == b"*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(b"*") {
        return name.len() >= prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(prefix);
    }
    pattern.eq_ignore_ascii_case(name)
}

fn paired_counts_response(counts: Vec<(Vec<u8>, i64)>) -> RespFrame {
    let mut response = Vec::with_capacity(counts.len() * 2);
    for (channel, count) in counts {
        response.push(RespFrame::Bulk(Some(BulkData::from_vec(channel))));
        response.push(RespFrame::Integer(count));
    }
    RespFrame::Array(Some(response))
}

fn array_of_bulk(values: Vec<Vec<u8>>) -> RespFrame {
    RespFrame::Array(Some(
        values
            .into_iter()
            .map(|value| RespFrame::Bulk(Some(BulkData::from_vec(value))))
            .collect(),
    ))
}

fn unsubscribe_all_response(
    count_after_all: i64,
    values: Vec<Vec<u8>>,
    kind: &'static [u8],
) -> RespFrame {
    let mut responses = Vec::with_capacity(values.len().max(1));
    if values.is_empty() {
        responses.push(unsubscribe_response(kind, None, count_after_all));
    } else {
        let mut remaining = count_after_all + values.len() as i64;
        for value in values {
            remaining -= 1;
            responses.push(unsubscribe_response(
                kind,
                Some(value.as_slice()),
                remaining,
            ));
        }
    }
    collapse_pubsub_responses(responses)
}

fn bulk_static(value: &'static [u8]) -> RespFrame {
    RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(value))))
}

fn subscription_response(kind: &'static [u8], value: &[u8], count: i64) -> RespFrame {
    RespFrame::Array(Some(vec![
        bulk_static(kind),
        RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(value)))),
        RespFrame::Integer(count),
    ]))
}

fn unsubscribe_response(kind: &'static [u8], value: Option<&[u8]>, count: i64) -> RespFrame {
    RespFrame::Array(Some(vec![
        bulk_static(kind),
        match value {
            Some(value) => RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(value)))),
            None => RespFrame::Bulk(None),
        },
        RespFrame::Integer(count),
    ]))
}

fn collapse_pubsub_responses(mut responses: Vec<RespFrame>) -> RespFrame {
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespFrame::Array(Some(responses))
    }
}

fn unknown_subcommand_error(subcommand: &[u8]) -> RespFrame {
    RespFrame::Error(format!(
        "ERR unknown subcommand '{}'.",
        String::from_utf8_lossy(subcommand)
    ))
}

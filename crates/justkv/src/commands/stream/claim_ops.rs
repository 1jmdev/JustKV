use crate::commands::stream::parse::{parse_count, parse_stream_id};
use crate::commands::util::{wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::engine::value::StreamId;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn xreadgroup(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 7 {
        return wrong_args("XREADGROUP");
    }
    if !args[1].eq_ignore_ascii_case(b"GROUP") {
        return RespFrame::Error("ERR syntax error".to_string());
    }
    let group = &args[2];
    let consumer = &args[3];

    let mut index = 4usize;
    let mut count = None;
    let mut noack = false;
    if index + 1 < args.len() && args[index].eq_ignore_ascii_case(b"COUNT") {
        count = match parse_count(&args[index + 1]) {
            Ok(value) => Some(value),
            Err(response) => return response,
        };
        index += 2;
    }
    if index + 1 < args.len() && args[index].eq_ignore_ascii_case(b"BLOCK") {
        index += 2;
    }
    if index < args.len() && args[index].eq_ignore_ascii_case(b"NOACK") {
        noack = true;
        index += 1;
    }
    if index >= args.len() || !args[index].eq_ignore_ascii_case(b"STREAMS") {
        return RespFrame::Error("ERR syntax error".to_string());
    }
    index += 1;

    let remaining = args.len() - index;
    if remaining == 0 || remaining % 2 != 0 {
        return RespFrame::Error("ERR Unbalanced XREADGROUP list of streams".to_string());
    }
    let stream_count = remaining / 2;

    let mut streams = Vec::with_capacity(stream_count);
    for offset in 0..stream_count {
        let key = args[index + offset].clone();
        let raw = &args[index + stream_count + offset];
        let id = if raw == b">" {
            StreamId {
                ms: u64::MAX,
                seq: u64::MAX,
            }
        } else {
            match parse_stream_id(raw) {
                Ok(value) => value,
                Err(response) => return response,
            }
        };
        streams.push((key, id));
    }

    match store.xreadgroup(group, consumer, &streams, count, noack) {
        Ok(items) => {
            if items.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Array(Some(
                    items
                        .into_iter()
                        .map(|(key, values)| {
                            RespFrame::Array(Some(vec![
                                RespFrame::Bulk(Some(BulkData::Arg(key))),
                                RespFrame::Array(Some(super::range_ops::format_items(values))),
                            ]))
                        })
                        .collect(),
                ))
            }
        }
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xclaim(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 6 {
        return wrong_args("XCLAIM");
    }

    let mut ids = Vec::with_capacity(args.len() - 5);
    for raw in &args[5..] {
        match parse_stream_id(raw) {
            Ok(value) => ids.push(value),
            Err(response) => return response,
        }
    }

    match store.xclaim(&args[1], &args[2], &args[3], &ids) {
        Ok(items) => RespFrame::Array(Some(super::range_ops::format_items(items))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xautoclaim(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 6 {
        return wrong_args("XAUTOCLAIM");
    }
    let start = match parse_stream_id(&args[5]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut count = 100usize;
    if args.len() >= 8 && args[6].eq_ignore_ascii_case(b"COUNT") {
        count = match parse_count(&args[7]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.xautoclaim(&args[1], &args[2], &args[3], start, count) {
        Ok((next, items)) => RespFrame::Array(Some(vec![
            super::parse::stream_id_to_bulk(next),
            RespFrame::Array(Some(super::range_ops::format_items(items))),
            RespFrame::Array(Some(vec![])),
        ])),
        Err(_) => wrong_type(),
    }
}

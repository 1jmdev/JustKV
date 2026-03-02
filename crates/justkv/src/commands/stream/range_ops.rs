use crate::commands::stream::parse::{parse_count, parse_stream_id, stream_id_to_bulk};
use crate::commands::util::{wrong_args, wrong_type, Args};
use crate::engine::store::stream::StreamRangeItem;
use crate::engine::store::Store;
use crate::engine::value::{CompactArg, StreamId};
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn xrange(store: &Store, args: &Args) -> RespFrame {
    range_common(store, args, false)
}

pub(crate) fn xrevrange(store: &Store, args: &Args) -> RespFrame {
    range_common(store, args, true)
}

fn range_common(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    if args.len() < 4 {
        return wrong_args(if reverse { "XREVRANGE" } else { "XRANGE" });
    }
    let start = match parse_stream_id(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let end = match parse_stream_id(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut count = None;
    if args.len() == 6 && args[4].eq_ignore_ascii_case(b"COUNT") {
        count = match parse_count(&args[5]) {
            Ok(value) => Some(value),
            Err(response) => return response,
        };
    } else if args.len() != 4 {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    match store.xrange(&args[1], start, end, reverse, count) {
        Ok(items) => RespFrame::Array(Some(format_items(items))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xread(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 4 {
        return wrong_args("XREAD");
    }

    let mut index = 1usize;
    let mut count = None;
    if args[index].eq_ignore_ascii_case(b"COUNT") {
        if index + 1 >= args.len() {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        count = match parse_count(&args[index + 1]) {
            Ok(value) => Some(value),
            Err(response) => return response,
        };
        index += 2;
    }
    if index < args.len() && args[index].eq_ignore_ascii_case(b"BLOCK") {
        if index + 1 >= args.len() {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        index += 2;
    }

    if index >= args.len() || !args[index].eq_ignore_ascii_case(b"STREAMS") {
        return RespFrame::Error("ERR syntax error".to_string());
    }
    index += 1;

    let remaining = args.len() - index;
    if remaining == 0 || remaining % 2 != 0 {
        return RespFrame::Error("ERR Unbalanced XREAD list of streams".to_string());
    }
    let stream_count = remaining / 2;

    let mut streams = Vec::with_capacity(stream_count);
    for offset in 0..stream_count {
        let key = args[index + offset].clone();
        let raw_id = &args[index + stream_count + offset];
        let id = if raw_id == b"$" {
            StreamId {
                ms: u64::MAX,
                seq: u64::MAX,
            }
        } else {
            match parse_stream_id(raw_id) {
                Ok(value) => value,
                Err(response) => return response,
            }
        };
        streams.push((key, id));
    }

    match store.xread(&streams, count) {
        Ok(values) => {
            if values.is_empty() {
                RespFrame::Bulk(None)
            } else {
                RespFrame::Array(Some(
                    values
                        .into_iter()
                        .map(|(key, items)| {
                            RespFrame::Array(Some(vec![
                                RespFrame::Bulk(Some(BulkData::Arg(key))),
                                RespFrame::Array(Some(format_items(items))),
                            ]))
                        })
                        .collect(),
                ))
            }
        }
        Err(_) => wrong_type(),
    }
}

pub(super) fn format_items(items: Vec<StreamRangeItem>) -> Vec<RespFrame> {
    items
        .into_iter()
        .map(|item| {
            let fields = item
                .fields
                .into_iter()
                .flat_map(|(field, value)| {
                    [
                        RespFrame::Bulk(Some(BulkData::Arg(field))),
                        RespFrame::Bulk(Some(BulkData::Value(value))),
                    ]
                })
                .collect();

            RespFrame::Array(Some(vec![
                stream_id_to_bulk(item.id),
                RespFrame::Array(Some(fields)),
            ]))
        })
        .collect()
}

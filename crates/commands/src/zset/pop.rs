use crate::util::{Args, eq_ascii, f64_to_bytes, int_error, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn zpop(store: &Store, args: &Args, max: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::pop::zpop");
    if args.len() != 2 && args.len() != 3 {
        return wrong_args(if max { "ZPOPMAX" } else { "ZPOPMIN" });
    }
    let count = if args.len() == 3 {
        match parse_usize(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        1
    };

    match if max {
        store.zpopmax(&args[1], count)
    } else {
        store.zpopmin(&args[1], count)
    } {
        Ok(Some(items)) => RespFrame::Array(Some(flatten_member_scores(items))),
        Ok(None) => RespFrame::Array(Some(Vec::new())),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bzpop(store: &Store, args: &Args, max: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::pop::bzpop");
    if args.len() < 3 {
        return wrong_args(if max { "BZPOPMAX" } else { "BZPOPMIN" });
    }
    if parse_timeout(&args[args.len() - 1]).is_err() {
        return int_error();
    }
    let keys = &args[1..args.len() - 1];
    match store.bzpop_edge(keys, max) {
        Ok(Some((key, member, score))) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::Arg(key))),
            RespFrame::Bulk(Some(BulkData::Arg(member))),
            RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
        ])),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zmpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::pop::zmpop");
    if args.len() < 4 {
        return wrong_args("ZMPOP");
    }
    let num_keys = match parse_usize(&args[1]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 2 + num_keys + 1 {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let keys_end = 2 + num_keys;
    let max = if eq_ascii(&args[keys_end], b"MAX") {
        true
    } else if eq_ascii(&args[keys_end], b"MIN") {
        false
    } else {
        return RespFrame::Error("ERR syntax error".to_string());
    };

    let mut count = 1usize;
    if args.len() > keys_end + 1 {
        if args.len() != keys_end + 3 || !eq_ascii(&args[keys_end + 1], b"COUNT") {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        count = match parse_usize(&args[keys_end + 2]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.zmpop(&args[2..keys_end], max, count) {
        Ok(Some((key, items))) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::Arg(key))),
            RespFrame::Array(Some(items_to_pairs(items))),
        ])),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bzmpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::pop::bzmpop");
    if args.len() < 5 {
        return wrong_args("BZMPOP");
    }
    if parse_timeout(&args[1]).is_err() {
        return int_error();
    }
    let num_keys = match parse_usize(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 3 + num_keys + 1 {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let keys_end = 3 + num_keys;
    let max = if eq_ascii(&args[keys_end], b"MAX") {
        true
    } else if eq_ascii(&args[keys_end], b"MIN") {
        false
    } else {
        return RespFrame::Error("ERR syntax error".to_string());
    };

    let mut count = 1usize;
    if args.len() > keys_end + 1 {
        if args.len() != keys_end + 3 || !eq_ascii(&args[keys_end + 1], b"COUNT") {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        count = match parse_usize(&args[keys_end + 2]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.zmpop(&args[3..keys_end], max, count) {
        Ok(Some((key, items))) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::Arg(key))),
            RespFrame::Array(Some(items_to_pairs(items))),
        ])),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("commands::zset::pop::parse_usize");
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

fn parse_timeout(raw: &[u8]) -> Result<f64, ()> {
    let _trace = profiler::scope("commands::zset::pop::parse_timeout");
    std::str::from_utf8(raw)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .ok_or(())
}

fn flatten_member_scores(items: Vec<(engine::value::CompactKey, f64)>) -> Vec<RespFrame> {
    let _trace = profiler::scope("commands::zset::pop::flatten_member_scores");
    let mut out = Vec::with_capacity(items.len() * 2);
    for (member, score) in items {
        out.push(RespFrame::Bulk(Some(BulkData::Arg(member))));
        out.push(RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(
            score,
        )))));
    }
    out
}

fn items_to_pairs(items: Vec<(engine::value::CompactKey, f64)>) -> Vec<RespFrame> {
    let _trace = profiler::scope("commands::zset::pop::items_to_pairs");
    items
        .into_iter()
        .map(|(member, score)| {
            RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::Arg(member))),
                RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
            ]))
        })
        .collect()
}

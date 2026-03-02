use crate::util::{Args, f64_to_bytes, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn zadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zadd");
    if args.len() < 4 || args.len() % 2 != 0 {
        return wrong_args("ZADD");
    }

    let mut pairs = Vec::with_capacity((args.len() - 2) / 2);
    for chunk in args[2..].chunks(2) {
        let score = match parse_f64(&chunk[0]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        pairs.push((score, chunk[1].clone()));
    }

    match store.zadd(&args[1], &pairs) {
        Ok(added) => RespFrame::Integer(added),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrem(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zrem");
    if args.len() < 3 {
        return wrong_args("ZREM");
    }
    match store.zrem(&args[1], &args[2..]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zcard(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zcard");
    if args.len() != 2 {
        return wrong_args("ZCARD");
    }
    match store.zcard(&args[1]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zcount(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zcount");
    if args.len() != 4 {
        return wrong_args("ZCOUNT");
    }
    let min = match parse_f64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let max = match parse_f64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zcount(&args[1], min, max) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zscore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zscore");
    if args.len() != 3 {
        return wrong_args("ZSCORE");
    }
    match store.zscore(&args[1], &args[2]) {
        Ok(Some(score)) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrank(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zrank");
    if args.len() != 3 {
        return wrong_args(if reverse { "ZREVRANK" } else { "ZRANK" });
    }
    match store.zrank(&args[1], &args[2], reverse) {
        Ok(Some(value)) => RespFrame::Integer(value),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zincrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zincrby");
    if args.len() != 4 {
        return wrong_args("ZINCRBY");
    }
    let increment = match parse_f64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zincrby(&args[1], increment, &args[3]) {
        Ok(score) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zmscore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zmscore");
    if args.len() < 3 {
        return wrong_args("ZMSCORE");
    }
    match store.zmscore(&args[1], &args[2..]) {
        Ok(scores) => RespFrame::Array(Some(
            scores
                .into_iter()
                .map(|score| {
                    RespFrame::Bulk(score.map(|value| BulkData::from_vec(f64_to_bytes(value))))
                })
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zremrangebyrank(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::core::zremrangebyrank");
    if args.len() != 4 {
        return wrong_args("ZREMRANGEBYRANK");
    }
    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let stop = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zremrangebyrank(&args[1], start, stop) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

fn parse_f64(raw: &[u8]) -> Result<f64, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::zset::core::parse_f64");
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| RespFrame::Error("ERR value is not a valid float".to_string())),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not a valid float".to_string(),
        )),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::zset::core::parse_i64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| crate::util::int_error()),
        Err(_) => Err(crate::util::int_error()),
    }
}

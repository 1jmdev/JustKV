use crate::util::{
    Args, f64_to_bytes, parse_i64_bytes, preencode_bulk_str, wrong_args, wrong_type,
};
use crate::zset::parse::{parse_score, parse_score_bound};
use engine::store::{LexBound, Store};
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

pub(crate) fn zadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zadd");
    if args.len() < 4 {
        return wrong_args("ZADD");
    }

    let mut index = 2usize;
    let mut nx = false;
    let mut xx = false;
    let mut gt = false;
    let mut lt = false;
    let mut incr = false;

    while index < args.len() {
        let option = args[index].as_slice();
        if option.eq_ignore_ascii_case(b"NX") {
            nx = true;
        } else if option.eq_ignore_ascii_case(b"XX") {
            xx = true;
        } else if option.eq_ignore_ascii_case(b"GT") {
            gt = true;
        } else if option.eq_ignore_ascii_case(b"LT") {
            lt = true;
        } else if option.eq_ignore_ascii_case(b"INCR") {
            incr = true;
        } else {
            break;
        }
        index += 1;
    }

    let pair_args = &args[index..];
    if pair_args.len() < 2 || pair_args.len() % 2 != 0 || (incr && pair_args.len() != 2) {
        return wrong_args("ZADD");
    }

    if nx && xx {
        return crate::util::syntax_error();
    }
    if gt && lt {
        return crate::util::syntax_error();
    }

    if incr {
        let increment = match parse_score(pair_args[0].slice()) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let member = &pair_args[1];
        let existing = match store.zscore(&args[1], member.as_slice()) {
            Ok(value) => value,
            Err(_) => return wrong_type(),
        };
        if nx && existing.is_some() {
            return RespFrame::Bulk(None);
        }
        if xx && existing.is_none() {
            return RespFrame::Bulk(None);
        }
        if let Some(current) = existing {
            if gt && increment <= 0.0 {
                return RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(current))));
            }
            if lt && increment >= 0.0 {
                return RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(current))));
            }
        }
        return match store.zincrby(&args[1], increment, member.as_slice()) {
            Ok(score) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
            Err(_) => wrong_type(),
        };
    }

    let mut added = 0i64;
    for chunk in pair_args.chunks(2) {
        let score = match parse_score(chunk[0].slice()) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let member = &chunk[1];
        let existing = match store.zscore(&args[1], member.as_slice()) {
            Ok(value) => value,
            Err(_) => return wrong_type(),
        };
        if nx && existing.is_some() {
            continue;
        }
        if xx && existing.is_none() {
            continue;
        }
        if let Some(current) = existing {
            if gt && score <= current {
                continue;
            }
            if lt && score >= current {
                continue;
            }
        }

        let pair = [(score, CompactArg::from_slice(member.as_slice()))];
        match store.zadd(&args[1], &pair) {
            Ok(value) => added += value,
            Err(_) => return wrong_type(),
        }
    }

    RespFrame::Integer(added)
}

pub(crate) fn zrem(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zrem");
    if args.len() < 3 {
        return wrong_args("ZREM");
    }
    match store.zrem(&args[1], &args[2..]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zcard(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zcard");
    if args.len() != 2 {
        return wrong_args("ZCARD");
    }
    match store.zcard(&args[1]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zcount(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zcount");
    if args.len() != 4 {
        return wrong_args("ZCOUNT");
    }
    let min = match parse_score_bound(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let max = match parse_score_bound(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zcount(&args[1], min.0, min.1, max.0, max.1) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zscore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zscore");
    if args.len() != 3 {
        return wrong_args("ZSCORE");
    }
    match store.zscore(&args[1], &args[2]) {
        Ok(Some(score)) => RespFrame::PreEncoded(preencode_bulk_str(
            std::str::from_utf8(&f64_to_bytes(score)).unwrap_or("0"),
        )),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrank(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zrank");
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
    let _trace = profiler::scope("commands::zset::core::zincrby");
    if args.len() != 4 {
        return wrong_args("ZINCRBY");
    }
    let increment = match parse_score(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zincrby(&args[1], increment, &args[3]) {
        Ok(score) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zmscore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zmscore");
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
    let _trace = profiler::scope("commands::zset::core::zremrangebyrank");
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

pub(crate) fn zlexcount(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zlexcount");
    if args.len() != 4 {
        return wrong_args("ZLEXCOUNT");
    }
    let min = match parse_lex_bound(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let max = match parse_lex_bound(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zlexcount(&args[1], min, max) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zremrangebylex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zremrangebylex");
    if args.len() != 4 {
        return wrong_args("ZREMRANGEBYLEX");
    }
    let min = match parse_lex_bound(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let max = match parse_lex_bound(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zremrangebylex(&args[1], min, max) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zremrangebyscore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::core::zremrangebyscore");
    if args.len() != 4 {
        return wrong_args("ZREMRANGEBYSCORE");
    }
    let min = match parse_score_bound(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let max = match parse_score_bound(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.zremrangebyscore(&args[1], min.0, min.1, max.0, max.1) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("commands::zset::core::parse_i64");
    parse_i64_bytes(raw).ok_or_else(crate::util::int_error)
}

fn parse_lex_bound(raw: &[u8]) -> Result<LexBound<'_>, RespFrame> {
    if raw == b"-" {
        return Ok(LexBound {
            value: None,
            inclusive: true,
        });
    }
    if raw == b"+" {
        return Ok(LexBound {
            value: None,
            inclusive: true,
        });
    }
    if let Some(value) = raw.strip_prefix(b"[") {
        return Ok(LexBound {
            value: Some(value),
            inclusive: true,
        });
    }
    if let Some(value) = raw.strip_prefix(b"(") {
        return Ok(LexBound {
            value: Some(value),
            inclusive: false,
        });
    }
    Err(RespFrame::Error(
        "ERR min or max not valid string range item".to_string(),
    ))
}

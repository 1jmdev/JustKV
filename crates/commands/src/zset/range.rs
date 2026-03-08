use crate::util::{
    Args, eq_ascii, f64_to_bytes, int_error, parse_i64_bytes, parse_u64_bytes, wrong_args,
    wrong_type,
};
use crate::zset::parse::parse_score_bound;
use engine::store::{LexBound, Store};
use protocol::types::{BulkData, RespFrame};
use types::value::CompactKey;

pub(crate) fn zrange(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::range::zrange");
    if args.len() < 4 {
        return wrong_args(if reverse { "ZREVRANGE" } else { "ZRANGE" });
    }

    let mut withscores = false;
    let mut rev_option = false;
    for option in &args[4..] {
        if eq_ascii(option, b"WITHSCORES") {
            withscores = true;
            continue;
        }
        if eq_ascii(option, b"REV") {
            rev_option = true;
            continue;
        }
        return crate::util::syntax_error();
    }

    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let stop = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.zrange(&args[1], start, stop, reverse || rev_option) {
        Ok(items) => RespFrame::Array(Some(format_items(items, withscores))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrange_by_score(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::range::zrange_by_score");
    if args.len() < 4 {
        return wrong_args(if reverse {
            "ZREVRANGEBYSCORE"
        } else {
            "ZRANGEBYSCORE"
        });
    }

    let first = match parse_score_bound(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let second = match parse_score_bound(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let (min, max) = if reverse {
        (second, first)
    } else {
        (first, second)
    };

    let mut withscores = false;
    let mut offset = 0usize;
    let mut count = None;
    let mut index = 4;
    while index < args.len() {
        if eq_ascii(&args[index], b"WITHSCORES") {
            withscores = true;
            index += 1;
            continue;
        }
        if eq_ascii(&args[index], b"LIMIT") {
            if index + 2 >= args.len() {
                return crate::util::syntax_error();
            }
            offset = match parse_usize(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            count = Some(match parse_usize(&args[index + 2]) {
                Ok(value) => value,
                Err(response) => return response,
            });
            index += 3;
            continue;
        }
        return crate::util::syntax_error();
    }

    match store.zrange_by_score(&args[1], min.0, min.1, max.0, max.1, reverse, offset, count) {
        Ok(items) => RespFrame::Array(Some(format_items(items, withscores))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrange_by_lex(store: &Store, args: &Args, reverse: bool) -> RespFrame {
    let _trace = profiler::scope("commands::zset::range::zrange_by_lex");
    if args.len() < 4 {
        return wrong_args(if reverse {
            "ZREVRANGEBYLEX"
        } else {
            "ZRANGEBYLEX"
        });
    }

    let (min, max) = if reverse {
        match (parse_lex_bound(&args[3]), parse_lex_bound(&args[2])) {
            (Ok(min), Ok(max)) => (min, max),
            (Err(response), _) | (_, Err(response)) => return response,
        }
    } else {
        match (parse_lex_bound(&args[2]), parse_lex_bound(&args[3])) {
            (Ok(min), Ok(max)) => (min, max),
            (Err(response), _) | (_, Err(response)) => return response,
        }
    };

    let mut offset = 0usize;
    let mut count = None;
    let mut index = 4;
    while index < args.len() {
        if !eq_ascii(&args[index], b"LIMIT") || index + 2 >= args.len() {
            return crate::util::syntax_error();
        }
        offset = match parse_usize(&args[index + 1]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        count = Some(match parse_usize(&args[index + 2]) {
            Ok(value) => value,
            Err(response) => return response,
        });
        index += 3;
    }

    match store.zrange_by_lex(&args[1], min, max, reverse, offset, count) {
        Ok(items) => RespFrame::Array(Some(
            items
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn zrangestore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::range::zrangestore");
    if args.len() != 5 {
        return wrong_args("ZRANGESTORE");
    }
    let start = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let stop = match parse_i64(&args[4]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store
        .zrange(&args[2], start, stop, false)
        .and_then(|items| store.zstore_items(&args[1], &items))
    {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

fn format_items(items: Vec<(CompactKey, f64)>, withscores: bool) -> Vec<RespFrame> {
    let _trace = profiler::scope("commands::zset::range::format_items");
    if withscores {
        let mut out = Vec::with_capacity(items.len() * 2);
        for (member, score) in items {
            out.push(RespFrame::Bulk(Some(BulkData::Arg(member))));
            out.push(RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(
                score,
            )))));
        }
        out
    } else {
        items
            .into_iter()
            .map(|(member, _)| RespFrame::Bulk(Some(BulkData::Arg(member))))
            .collect()
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
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

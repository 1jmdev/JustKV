use crate::util::{Args, eq_ascii, parse_i64_bytes, wrong_args, wrong_type};
use engine::store::{MSetExExistCondition, SharedTtl, Store};
use protocol::types::RespFrame;

pub(crate) fn mget(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::multi::mget");
    if args.len() < 2 {
        return wrong_args("MGET");
    }
    match store.mget_encode(&args[1..]) {
        Ok(bytes) => RespFrame::PreEncoded(bytes),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn mset(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::multi::mset");
    if args.len() < 3 || !(args.len() - 1).is_multiple_of(2) {
        return wrong_args("MSET");
    }
    store.mset_args(&args[1..]);
    RespFrame::ok()
}

pub(crate) fn msetex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::multi::msetex");
    if args.len() < 4 {
        return wrong_args("MSETEX");
    }

    if let Ok(pair_count) = parse_numkeys(&args[1]) {
        if pair_count == 0 {
            return wrong_args("MSETEX");
        }

        let pair_args_len = match pair_count.checked_mul(2) {
            Some(value) => value,
            None => return RespFrame::error_static("ERR value is not an integer or out of range"),
        };
        let options_start = match 2usize.checked_add(pair_args_len) {
            Some(value) => value,
            None => return RespFrame::error_static("ERR value is not an integer or out of range"),
        };
        if args.len() < options_start {
            return crate::util::syntax_error();
        }

        let mut pairs = Vec::with_capacity(pair_count);
        for chunk in args[2..options_start].chunks_exact(2) {
            pairs.push((chunk[0].clone(), chunk[1].clone()));
        }

        let (condition, ttl) = match parse_msetex_options(args, options_start) {
            Ok(value) => value,
            Err(response) => return response,
        };

        return RespFrame::Integer(store.msetex(&pairs, condition, ttl));
    }

    let options_start = find_msetex_options_start(args);
    let pair_args_len = options_start - 1;
    if pair_args_len == 0 || !pair_args_len.is_multiple_of(2) {
        return RespFrame::error_static("ERR value is not an integer or out of range");
    }

    let mut pairs = Vec::with_capacity(pair_args_len / 2);
    for chunk in args[1..options_start].chunks_exact(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }

    let (condition, ttl) = match parse_msetex_options(args, options_start) {
        Ok(value) => value,
        Err(response) => return response,
    };

    if store.msetex(&pairs, condition, ttl) == 0 {
        RespFrame::Bulk(None)
    } else {
        RespFrame::ok()
    }
}

pub(crate) fn msetnx(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::multi::msetnx");
    if args.len() < 3 || !(args.len() - 1).is_multiple_of(2) {
        return wrong_args("MSETNX");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }
    RespFrame::Integer(store.msetnx(pairs) as i64)
}

fn parse_msetex_options(
    args: &Args,
    mut index: usize,
) -> Result<(MSetExExistCondition, SharedTtl), RespFrame> {
    let mut condition = MSetExExistCondition::Any;
    let mut ttl = SharedTtl::None;

    while index < args.len() {
        let option = args[index].as_slice();
        if eq_ascii(option, b"NX") {
            if condition != MSetExExistCondition::Any {
                return Err(crate::util::syntax_error());
            }
            condition = MSetExExistCondition::Nx;
            index += 1;
            continue;
        }
        if eq_ascii(option, b"XX") {
            if condition != MSetExExistCondition::Any {
                return Err(crate::util::syntax_error());
            }
            condition = MSetExExistCondition::Xx;
            index += 1;
            continue;
        }
        if eq_ascii(option, b"KEEPTTL") {
            if ttl != SharedTtl::None {
                return Err(crate::util::syntax_error());
            }
            ttl = SharedTtl::Keep;
            index += 1;
            continue;
        }

        index += 1;
        if index >= args.len() {
            return Err(crate::util::syntax_error());
        }

        let value = parse_positive_u64(&args[index], "msetex")?;
        if ttl != SharedTtl::None {
            return Err(crate::util::syntax_error());
        }

        if eq_ascii(option, b"EX") {
            ttl = SharedTtl::RelativeMs(value.saturating_mul(1000));
        } else if eq_ascii(option, b"PX") {
            ttl = SharedTtl::RelativeMs(value);
        } else if eq_ascii(option, b"EXAT") {
            ttl = SharedTtl::AbsoluteUnixMs(value.saturating_mul(1000));
        } else if eq_ascii(option, b"PXAT") {
            ttl = SharedTtl::AbsoluteUnixMs(value);
        } else {
            return Err(crate::util::syntax_error());
        }

        index += 1;
    }

    Ok((condition, ttl))
}

fn find_msetex_options_start(args: &Args) -> usize {
    let mut index = 1;
    while index < args.len() {
        let option = args[index].as_slice();
        if is_msetex_option(option) {
            return index;
        }
        index += 1;
    }
    args.len()
}

fn is_msetex_option(value: &[u8]) -> bool {
    eq_ascii(value, b"NX")
        || eq_ascii(value, b"XX")
        || eq_ascii(value, b"EX")
        || eq_ascii(value, b"PX")
        || eq_ascii(value, b"EXAT")
        || eq_ascii(value, b"PXAT")
        || eq_ascii(value, b"KEEPTTL")
}

fn parse_numkeys(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_i64_bytes(raw)
        .ok_or_else(|| RespFrame::error_static("ERR value is not an integer or out of range"))?;
    usize::try_from(value)
        .map_err(|_| RespFrame::error_static("ERR value is not an integer or out of range"))
}

fn parse_positive_u64(raw: &[u8], command: &str) -> Result<u64, RespFrame> {
    let value = parse_i64_bytes(raw)
        .ok_or_else(|| RespFrame::error_static("ERR value is not an integer or out of range"))?;
    if value <= 0 {
        return Err(RespFrame::Error(format!(
            "ERR invalid expire time in '{command}' command"
        )));
    }
    Ok(value as u64)
}

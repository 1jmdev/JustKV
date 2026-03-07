use crate::util::{eq_ascii, int_error, parse_u64_bytes, wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactKey;

pub(crate) fn sinter(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sinter");
    if args.len() < 2 {
        return wrong_args("SINTER");
    }
    members_response(store.sinter(&args[1..]))
}

pub(crate) fn sinterstore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sinterstore");
    if args.len() < 3 {
        return wrong_args("SINTERSTORE");
    }
    match store.sinterstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sunion(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sunion");
    if args.len() < 2 {
        return wrong_args("SUNION");
    }
    members_response(store.sunion(&args[1..]))
}

pub(crate) fn sunionstore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sunionstore");
    if args.len() < 3 {
        return wrong_args("SUNIONSTORE");
    }
    match store.sunionstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sdiff(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sdiff");
    if args.len() < 2 {
        return wrong_args("SDIFF");
    }
    members_response(store.sdiff(&args[1..]))
}

pub(crate) fn sdiffstore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sdiffstore");
    if args.len() < 3 {
        return wrong_args("SDIFFSTORE");
    }
    match store.sdiffstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sintercard(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::sintercard");
    if args.len() < 3 {
        return wrong_args("SINTERCARD");
    }
    let num_keys = match parse_usize(&args[1]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 2 + num_keys {
        return crate::util::syntax_error();
    }

    let keys_end = 2 + num_keys;
    let mut limit = None;
    if args.len() > keys_end {
        if args.len() != keys_end + 2 || !eq_ascii(&args[keys_end], b"LIMIT") {
            return crate::util::syntax_error();
        }
        limit = Some(match parse_usize(&args[keys_end + 1]) {
            Ok(value) => value,
            Err(response) => return response,
        });
    }

    match store.sintercard(&args[2..keys_end], limit) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

fn members_response(result: Result<Vec<CompactKey>, ()>) -> RespFrame {
    let _trace = profiler::scope("commands::set::algebra::members_response");
    match result {
        Ok(members) => RespFrame::Array(Some(
            members
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

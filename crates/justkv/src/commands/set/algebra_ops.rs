use crate::commands::util::{eq_ascii, int_error, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn sinter(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SINTER");
    }
    members_response(store.sinter(&args[1..]))
}

pub(crate) fn sinterstore(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SINTERSTORE");
    }
    match store.sinterstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sunion(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SUNION");
    }
    members_response(store.sunion(&args[1..]))
}

pub(crate) fn sunionstore(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SUNIONSTORE");
    }
    match store.sunionstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sdiff(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("SDIFF");
    }
    members_response(store.sdiff(&args[1..]))
}

pub(crate) fn sdiffstore(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SDIFFSTORE");
    }
    match store.sdiffstore(&args[1], &args[2..]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sintercard(store: &Store, args: &Args) -> RespFrame {
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
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let keys_end = 2 + num_keys;
    let mut limit = None;
    if args.len() > keys_end {
        if args.len() != keys_end + 2 || !eq_ascii(&args[keys_end], b"LIMIT") {
            return RespFrame::Error("ERR syntax error".to_string());
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

fn members_response(result: Result<Vec<crate::engine::value::CompactKey>, ()>) -> RespFrame {
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
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

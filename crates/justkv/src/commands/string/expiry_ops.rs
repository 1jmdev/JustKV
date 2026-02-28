use std::time::Duration;

use crate::commands::util::{eq_ascii, int_error, wrong_args, Args};
use crate::engine::store::{GetExMode, Store};
use crate::engine::value::CompactArg;
use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"SETEX") {
        return Some(setex(store, args));
    }
    if eq_ascii(command, b"PSETEX") {
        return Some(psetex(store, args));
    }
    if eq_ascii(command, b"GETEX") {
        return Some(getex(store, args));
    }
    None
}

fn setex(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("SETEX");
    }

    let ttl_seconds = match parse_positive_u64(&args[2], "setex") {
        Ok(value) => value,
        Err(response) => return response,
    };

    store.set(&args[1], &args[3], Some(Duration::from_secs(ttl_seconds)));
    RespFrame::ok()
}

fn psetex(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("PSETEX");
    }

    let ttl_millis = match parse_positive_u64(&args[2], "psetex") {
        Ok(value) => value,
        Err(response) => return response,
    };

    store.set(&args[1], &args[3], Some(Duration::from_millis(ttl_millis)));
    RespFrame::ok()
}

fn getex(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("GETEX");
    }

    let mode = match parse_getex_mode(args) {
        Ok(value) => value,
        Err(response) => return response,
    };

    RespFrame::Bulk(
        store
            .getex(&args[1], mode)
            .map(|value| BulkData::Arg(CompactArg::from_vec(value))),
    )
}

fn parse_getex_mode(args: &Args) -> Result<GetExMode, RespFrame> {
    if args.len() == 2 {
        return Ok(GetExMode::KeepTtl);
    }

    if args.len() == 3 && eq_ascii(&args[2], b"PERSIST") {
        return Ok(GetExMode::Persist);
    }

    if args.len() != 4 {
        return Err(RespFrame::Error("ERR syntax error".to_string()));
    }

    let option = args[2].as_slice();
    if eq_ascii(option, b"EX") {
        return parse_positive_u64(&args[3], "getex").map(GetExMode::Ex);
    }
    if eq_ascii(option, b"PX") {
        return parse_positive_u64(&args[3], "getex").map(GetExMode::Px);
    }
    if eq_ascii(option, b"EXAT") {
        return parse_positive_u64(&args[3], "getex").map(GetExMode::ExAt);
    }
    if eq_ascii(option, b"PXAT") {
        return parse_positive_u64(&args[3], "getex").map(GetExMode::PxAt);
    }

    Err(RespFrame::Error("ERR syntax error".to_string()))
}

fn parse_positive_u64(raw: &[u8], command: &str) -> Result<u64, RespFrame> {
    let value = match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error())?,
        Err(_) => return Err(int_error()),
    };

    if value == 0 {
        return Err(RespFrame::Error(format!(
            "ERR invalid expire time in '{command}' command"
        )));
    }

    Ok(value)
}

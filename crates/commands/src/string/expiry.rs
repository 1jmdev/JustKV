use std::time::Duration;

use crate::util::{Args, eq_ascii, parse_i64_bytes, wrong_args, wrong_type};
use engine::store::{GetExMode, Store};
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

pub(crate) fn setex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::expiry::setex");
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

pub(crate) fn psetex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::expiry::psetex");
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

pub(crate) fn getex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::expiry::getex");
    if args.len() < 2 {
        return wrong_args("GETEX");
    }

    let mode = match parse_getex_mode(args) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.getex(&args[1], mode) {
        Ok(value) => RespFrame::Bulk(value.map(|value| BulkData::Arg(CompactArg::from_vec(value)))),
        Err(_) => wrong_type(),
    }
}

fn parse_getex_mode(args: &Args) -> Result<GetExMode, RespFrame> {
    let _trace = profiler::scope("commands::string::expiry::parse_getex_mode");
    if args.len() == 2 {
        return Ok(GetExMode::KeepTtl);
    }

    if args.len() == 3 && eq_ascii(&args[2], b"PERSIST") {
        return Ok(GetExMode::Persist);
    }

    if args.len() != 4 {
        return Err(crate::util::syntax_error());
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

    Err(crate::util::syntax_error())
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

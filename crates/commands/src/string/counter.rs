use crate::util::{Args, int_error, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn incr(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::incr");
    if args.len() != 2 {
        return wrong_args("INCR");
    }
    match store.incr(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => match store.value_kind(&args[1]) {
            Some(kind) if kind != "string" => wrong_type(),
            _ => int_error(),
        },
    }
}

pub(crate) fn incrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::incrby");
    if args.len() != 3 {
        return wrong_args("INCRBY");
    }
    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by(&args[1], delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => match store.value_kind(&args[1]) {
            Some(kind) if kind != "string" => wrong_type(),
            _ => int_error(),
        },
    }
}

pub(crate) fn decr(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::decr");
    if args.len() != 2 {
        return wrong_args("DECR");
    }
    match store.incr_by(&args[1], -1) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => match store.value_kind(&args[1]) {
            Some(kind) if kind != "string" => wrong_type(),
            _ => int_error(),
        },
    }
}

pub(crate) fn decrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::decrby");
    if args.len() != 3 {
        return wrong_args("DECRBY");
    }
    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by(&args[1], -delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => match store.value_kind(&args[1]) {
            Some(kind) if kind != "string" => wrong_type(),
            _ => int_error(),
        },
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("commands::string::counter::parse_i64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

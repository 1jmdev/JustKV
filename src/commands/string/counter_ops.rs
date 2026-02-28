use crate::commands::util::{int_error, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"INCR" => Some(incr(store, args)),
        b"INCRBY" => Some(incrby(store, args)),
        b"DECR" => Some(decr(store, args)),
        b"DECRBY" => Some(decrby(store, args)),
        _ => None,
    }
}

fn incr(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("INCR");
    }
    match store.incr(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => int_error(),
    }
}

fn incrby(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("INCRBY");
    }

    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by(&args[1], delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => int_error(),
    }
}

fn decr(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("DECR");
    }

    match store.incr_by(&args[1], -1) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => int_error(),
    }
}

fn decrby(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("DECRBY");
    }

    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by(&args[1], -delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => int_error(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

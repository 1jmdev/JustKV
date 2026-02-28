use crate::commands::util::{eq_ascii, int_error, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"INCR") {
        return Some(incr(store, args));
    }
    if eq_ascii(command, b"INCRBY") {
        return Some(incrby(store, args));
    }
    if eq_ascii(command, b"DECR") {
        return Some(decr(store, args));
    }
    if eq_ascii(command, b"DECRBY") {
        return Some(decrby(store, args));
    }
    None
}

fn incr(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("INCR");
    }
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
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
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
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
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
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
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
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

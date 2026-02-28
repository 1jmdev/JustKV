use std::time::Duration;

use crate::commands::util::{int_error, upper, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"GET" => Some(get(store, args)),
        b"SET" => Some(set(store, args)),
        b"SETNX" => Some(setnx(store, args)),
        b"GETSET" => Some(getset(store, args)),
        b"GETDEL" => Some(getdel(store, args)),
        b"APPEND" => Some(append(store, args)),
        b"STRLEN" => Some(strlen(store, args)),
        b"INCR" => Some(incr(store, args)),
        b"INCRBY" => Some(incrby(store, args)),
        b"DECR" => Some(decr(store, args)),
        b"DECRBY" => Some(decrby(store, args)),
        b"MGET" => Some(mget(store, args)),
        b"MSET" => Some(mset(store, args)),
        b"MSETNX" => Some(msetnx(store, args)),
        _ => None,
    }
}

fn get(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GET");
    }
    RespFrame::Bulk(store.get(&args[1]))
}

fn set(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SET");
    }

    let mut ttl = None;
    let mut nx = false;
    let mut xx = false;
    let mut return_old = false;
    let mut index = 3;

    while index < args.len() {
        let opt = upper(&args[index]);
        match opt.as_slice() {
            b"NX" => nx = true,
            b"XX" => xx = true,
            b"GET" => return_old = true,
            b"EX" => {
                index += 1;
                if index >= args.len() {
                    return wrong_args("SET");
                }
                let seconds = match parse_u64(&args[index]) {
                    Ok(value) => value,
                    Err(response) => return response,
                };
                ttl = Some(Duration::from_secs(seconds));
            }
            b"PX" => {
                index += 1;
                if index >= args.len() {
                    return wrong_args("SET");
                }
                let millis = match parse_u64(&args[index]) {
                    Ok(value) => value,
                    Err(response) => return response,
                };
                ttl = Some(Duration::from_millis(millis));
            }
            _ => return RespFrame::Error("ERR syntax error".to_string()),
        }
        index += 1;
    }

    if nx && xx {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let key = args[1].clone();
    let value = args[2].clone();

    let old_value = if return_old { store.get(&key) } else { None };
    let success = if nx {
        store.setnx(key, value, ttl)
    } else if xx {
        store.setxx(key, value, ttl)
    } else {
        store.set(key, value, ttl);
        true
    };

    if !success {
        return RespFrame::Bulk(None);
    }

    if return_old {
        RespFrame::Bulk(old_value)
    } else {
        RespFrame::ok()
    }
}

fn setnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("SETNX");
    }
    RespFrame::Integer(store.setnx(args[1].clone(), args[2].clone(), None) as i64)
}

fn getset(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("GETSET");
    }
    RespFrame::Bulk(store.getset(args[1].clone(), args[2].clone()))
}

fn getdel(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GETDEL");
    }
    RespFrame::Bulk(store.getdel(&args[1]))
}

fn append(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("APPEND");
    }
    RespFrame::Integer(store.append(args[1].clone(), &args[2]) as i64)
}

fn strlen(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("STRLEN");
    }
    RespFrame::Integer(store.strlen(&args[1]) as i64)
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

fn mget(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("MGET");
    }
    let keys = args[1..].to_vec();
    let values = store.mget(&keys);
    RespFrame::Array(Some(values.into_iter().map(RespFrame::Bulk).collect()))
}

fn mset(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSET");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }
    store.mset(&pairs);
    RespFrame::ok()
}

fn msetnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSETNX");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }
    RespFrame::Integer(store.msetnx(&pairs) as i64)
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

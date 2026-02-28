use std::time::Duration;

use crate::commands::util::{upper, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"GET" => Some(get(store, args)),
        b"SET" => Some(set(store, args)),
        b"SETNX" => Some(setnx(store, args)),
        b"GETSET" => Some(getset(store, args)),
        b"GETDEL" => Some(getdel(store, args)),
        b"APPEND" => Some(append(store, args)),
        b"STRLEN" => Some(strlen(store, args)),
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

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            RespFrame::Error("ERR value is not an integer or out of range".to_string())
        }),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not an integer or out of range".to_string(),
        )),
    }
}

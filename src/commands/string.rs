use std::time::Duration;

use crate::commands::util::{wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"GET" => Some(get(store, args)),
        b"SET" => Some(set(store, args)),
        b"INCR" => Some(incr(store, args)),
        b"MGET" => Some(mget(store, args)),
        b"MSET" => Some(mset(store, args)),
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
    if args.len() != 3 {
        return wrong_args("SET");
    }
    store.set(args[1].clone(), args[2].clone(), None::<Duration>);
    RespFrame::ok()
}

fn incr(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("INCR");
    }
    match store.incr(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR value is not an integer or out of range".to_string()),
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

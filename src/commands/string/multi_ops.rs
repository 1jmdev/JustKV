use crate::commands::util::{wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"MGET" => Some(mget(store, args)),
        b"MSET" => Some(mset(store, args)),
        b"MSETNX" => Some(msetnx(store, args)),
        _ => None,
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

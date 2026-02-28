use crate::commands::util::{Args, eq_ascii, wrong_args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"MGET") {
        return Some(mget(store, args));
    }
    if eq_ascii(command, b"MSET") {
        return Some(mset(store, args));
    }
    if eq_ascii(command, b"MSETNX") {
        return Some(msetnx(store, args));
    }
    None
}

fn mget(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("MGET");
    }
    RespFrame::Array(Some(
        args[1..]
            .iter()
            .map(|key| store.get(key))
            .map(|value| RespFrame::Bulk(value.map(BulkData::Value)))
            .collect(),
    ))
}

fn mset(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSET");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].to_vec(), chunk[1].to_vec()));
    }
    store.mset(pairs);
    RespFrame::ok()
}

fn msetnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSETNX");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].to_vec(), chunk[1].to_vec()));
    }
    RespFrame::Integer(store.msetnx(pairs) as i64)
}

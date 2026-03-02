use crate::util::{Args, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn mget(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("MGET");
    }
    match store.mget(&args[1..]) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(value.map(BulkData::Value)))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn mset(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSET");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }
    store.mset(pairs);
    RespFrame::ok()
}

pub(crate) fn msetnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return wrong_args("MSETNX");
    }
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }
    RespFrame::Integer(store.msetnx(pairs) as i64)
}

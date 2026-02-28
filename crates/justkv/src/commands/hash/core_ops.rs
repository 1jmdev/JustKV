use crate::commands::util::{eq_ascii, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"HSET") || eq_ascii(command, b"HMSET") {
        return Some(hset(store, command, args));
    }
    if eq_ascii(command, b"HSETNX") {
        return Some(hsetnx(store, args));
    }
    if eq_ascii(command, b"HGET") {
        return Some(hget(store, args));
    }
    if eq_ascii(command, b"HMGET") {
        return Some(hmget(store, args));
    }
    if eq_ascii(command, b"HGETALL") {
        return Some(hgetall(store, args));
    }
    if eq_ascii(command, b"HDEL") {
        return Some(hdel(store, args));
    }
    if eq_ascii(command, b"HEXISTS") {
        return Some(hexists(store, args));
    }
    if eq_ascii(command, b"HKEYS") {
        return Some(hkeys(store, args));
    }
    if eq_ascii(command, b"HVALS") {
        return Some(hvals(store, args));
    }
    if eq_ascii(command, b"HLEN") {
        return Some(hlen(store, args));
    }
    if eq_ascii(command, b"HSTRLEN") {
        return Some(hstrlen(store, args));
    }
    None
}

fn hset(store: &Store, command: &[u8], args: &Args) -> RespFrame {
    if args.len() < 4 || args.len() % 2 != 0 {
        return wrong_args(if eq_ascii(command, b"HMSET") {
            "HMSET"
        } else {
            "HSET"
        });
    }

    let pairs: Vec<_> = args[2..]
        .chunks(2)
        .map(|chunk| (chunk[0].clone(), chunk[1].clone()))
        .collect();

    match store.hset(&args[1], &pairs) {
        Ok(created) => RespFrame::Integer(created),
        Err(_) => wrong_type(),
    }
}

fn hsetnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("HSETNX");
    }
    match store.hsetnx(&args[1], &args[2], &args[3]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

fn hget(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("HGET");
    }
    match store.hget(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

fn hmget(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("HMGET");
    }
    match store.hmget(&args[1], &args[2..]) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(value.map(BulkData::Value)))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

fn hgetall(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("HGETALL");
    }
    match store.hgetall(&args[1]) {
        Ok(pairs) => {
            let mut out = Vec::with_capacity(pairs.len() * 2);
            for (field, value) in pairs {
                out.push(RespFrame::Bulk(Some(BulkData::Arg(field))));
                out.push(RespFrame::Bulk(Some(BulkData::Value(value))));
            }
            RespFrame::Array(Some(out))
        }
        Err(_) => wrong_type(),
    }
}

fn hdel(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("HDEL");
    }
    match store.hdel(&args[1], &args[2..]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

fn hexists(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("HEXISTS");
    }
    match store.hexists(&args[1], &args[2]) {
        Ok(exists) => RespFrame::Integer(exists),
        Err(_) => wrong_type(),
    }
}

fn hkeys(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("HKEYS");
    }
    match store.hkeys(&args[1]) {
        Ok(fields) => RespFrame::Array(Some(
            fields
                .into_iter()
                .map(|field| RespFrame::Bulk(Some(BulkData::Arg(field))))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

fn hvals(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("HVALS");
    }
    match store.hvals(&args[1]) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(Some(BulkData::Value(value))))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

fn hlen(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("HLEN");
    }
    match store.hlen(&args[1]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

fn hstrlen(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("HSTRLEN");
    }
    match store.hstrlen(&args[1], &args[2]) {
        Ok(len) => RespFrame::Integer(len as i64),
        Err(_) => wrong_type(),
    }
}

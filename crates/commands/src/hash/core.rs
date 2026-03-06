use crate::util::{wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn hset(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hset");
    if args.len() < 4 || args.len() % 2 != 0 {
        return wrong_args("HSET");
    }

    match store.hset_args(&args[1], &args[2..]) {
        Ok(created) => RespFrame::Integer(created),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hmset(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hmset");
    if args.len() < 4 || args.len() % 2 != 0 {
        return wrong_args("HMSET");
    }

    match store.hset_args(&args[1], &args[2..]) {
        Ok(_) => RespFrame::ok(),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hsetnx(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hsetnx");
    if args.len() != 4 {
        return wrong_args("HSETNX");
    }
    match store.hsetnx(&args[1], &args[2], &args[3]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hget(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hget");
    if args.len() != 3 {
        return wrong_args("HGET");
    }
    match store.hget(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hmget(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hmget");
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

pub(crate) fn hgetall(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hgetall");
    if args.len() != 2 {
        return wrong_args("HGETALL");
    }
    match store.hgetall_encode(args[1].slice()) {
        Ok(bytes) => RespFrame::PreEncoded(bytes),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hdel(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hdel");
    if args.len() < 3 {
        return wrong_args("HDEL");
    }
    match store.hdel(&args[1], &args[2..]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hexists(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hexists");
    if args.len() != 3 {
        return wrong_args("HEXISTS");
    }
    match store.hexists(&args[1], &args[2]) {
        Ok(exists) => RespFrame::Integer(exists),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hkeys(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hkeys");
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

pub(crate) fn hvals(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hvals");
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

pub(crate) fn hlen(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hlen");
    if args.len() != 2 {
        return wrong_args("HLEN");
    }
    match store.hlen(&args[1]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn hstrlen(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::hash::core::hstrlen");
    if args.len() != 3 {
        return wrong_args("HSTRLEN");
    }
    match store.hstrlen(&args[1], &args[2]) {
        Ok(len) => RespFrame::Integer(len as i64),
        Err(_) => wrong_type(),
    }
}

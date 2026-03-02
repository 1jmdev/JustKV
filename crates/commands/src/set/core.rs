use crate::util::{Args, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn sadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::sadd");
    if args.len() < 3 {
        return wrong_args("SADD");
    }
    match store.sadd(&args[1], &args[2..]) {
        Ok(added) => RespFrame::Integer(added),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn srem(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::srem");
    if args.len() < 3 {
        return wrong_args("SREM");
    }
    match store.srem(&args[1], &args[2..]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn smembers(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::smembers");
    if args.len() != 2 {
        return wrong_args("SMEMBERS");
    }
    match store.smembers(&args[1]) {
        Ok(members) => RespFrame::Array(Some(
            members
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn sismember(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::sismember");
    if args.len() != 3 {
        return wrong_args("SISMEMBER");
    }
    match store.sismember(&args[1], &args[2]) {
        Ok(found) => RespFrame::Integer(found),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn scard(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::scard");
    if args.len() != 2 {
        return wrong_args("SCARD");
    }
    match store.scard(&args[1]) {
        Ok(size) => RespFrame::Integer(size),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn smove(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::set::core::smove");
    if args.len() != 4 {
        return wrong_args("SMOVE");
    }
    match store.smove(&args[1], &args[2], &args[3]) {
        Ok(result) => RespFrame::Integer(result),
        Err(_) => wrong_type(),
    }
}

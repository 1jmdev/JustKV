use crate::util::{Args, int_error, parse_u64_bytes, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn lpush(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::lpush");
    if args.len() < 3 {
        return wrong_args("LPUSH");
    }
    match store.lpush(&args[1], &args[2..]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lpushx(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::lpushx");
    if args.len() < 3 {
        return wrong_args("LPUSHX");
    }
    match store.lpushx(&args[1], &args[2..]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn rpush(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::rpush");
    if args.len() < 3 {
        return wrong_args("RPUSH");
    }
    match store.rpush(&args[1], &args[2..]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn rpushx(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::rpushx");
    if args.len() < 3 {
        return wrong_args("RPUSHX");
    }
    match store.rpushx(&args[1], &args[2..]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::lpop");
    pop(store, args, true)
}

pub(crate) fn rpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::rpop");
    pop(store, args, false)
}

fn pop(store: &Store, args: &Args, left: bool) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::pop");
    if args.len() != 2 && args.len() != 3 {
        return wrong_args(if left { "LPOP" } else { "RPOP" });
    }

    let has_count = args.len() == 3;
    let count = if has_count {
        match parse_usize(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        1
    };

    let result = if left {
        store.lpop(&args[1], count)
    } else {
        store.rpop(&args[1], count)
    };

    match result {
        Ok(Some(values)) if has_count => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(Some(BulkData::Value(value))))
                .collect(),
        )),
        Ok(Some(mut values)) => RespFrame::Bulk(values.pop().map(BulkData::Value)),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn llen(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::llen");
    if args.len() != 2 {
        return wrong_args("LLEN");
    }
    match store.llen(&args[1]) {
        Ok(len) => RespFrame::Integer(len),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lrem(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::core::lrem");
    if args.len() != 4 {
        return wrong_args("LREM");
    }
    let count = match crate::util::parse_i64_bytes(&args[2]) {
        Some(value) => value,
        None => return int_error(),
    };
    match store.lrem(&args[1], count, &args[3]) {
        Ok(removed) => RespFrame::Integer(removed),
        Err(_) => wrong_type(),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

use crate::util::{int_error, parse_u64_bytes, wrong_args, wrong_type, Args};
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

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

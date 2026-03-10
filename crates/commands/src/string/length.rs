use crate::util::{int_error, parse_i64_bytes, parse_u64_bytes, wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn append(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::length::append");
    if args.len() != 3 {
        return wrong_args("APPEND");
    }
    match store.append(&args[1], &args[2]) {
        Ok(value) => RespFrame::Integer(value as i64),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn strlen(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::length::strlen");
    if args.len() != 2 {
        return wrong_args("STRLEN");
    }
    match store.strlen(&args[1]) {
        Ok(value) => RespFrame::Integer(value as i64),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn setrange(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::length::setrange");
    if args.len() != 4 {
        return wrong_args("SETRANGE");
    }
    let offset = match parse_usize(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.setrange(&args[1], offset, &args[3]) {
        Ok(value) => RespFrame::Integer(value as i64),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn getrange(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::length::getrange");
    if args.len() != 4 {
        return wrong_args("GETRANGE");
    }
    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let end = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.getrange(&args[1], start, end) {
        Ok(value) => RespFrame::Bulk(Some(BulkData::from_vec(value))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn substr(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::length::substr");
    if args.len() != 4 {
        return wrong_args("SUBSTR");
    }
    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let end = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.substr(&args[1], start, end) {
        Ok(value) => RespFrame::Bulk(Some(BulkData::from_vec(value))),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(value).map_err(|_| RespFrame::error_static("ERR offset is out of range"))
}

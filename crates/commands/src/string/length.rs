use crate::util::{Args, int_error, wrong_args, wrong_type};
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

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("commands::string::length::parse_i64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("commands::string::length::parse_usize");
    let value = match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error())?,
        Err(_) => return Err(int_error()),
    };
    usize::try_from(value).map_err(|_| RespFrame::Error("ERR offset is out of range".to_string()))
}

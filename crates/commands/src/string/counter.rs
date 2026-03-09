use crate::util::{f64_to_bytes, int_error, parse_i64_bytes, wrong_args, wrong_type, Args};
use engine::store::{Store, StringIntOpError};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn incr(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::incr");
    if args.len() != 2 {
        return wrong_args("INCR");
    }
    match store.incr(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(StringIntOpError::WrongType) => wrong_type(),
        Err(StringIntOpError::InvalidInteger) => int_error(),
        Err(StringIntOpError::Overflow) => {
            RespFrame::Error("ERR increment or decrement would overflow".to_string())
        }
    }
}

pub(crate) fn incrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::incrby");
    if args.len() != 3 {
        return wrong_args("INCRBY");
    }
    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by(&args[1], delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(StringIntOpError::WrongType) => wrong_type(),
        Err(StringIntOpError::InvalidInteger) => int_error(),
        Err(StringIntOpError::Overflow) => {
            RespFrame::Error("ERR increment or decrement would overflow".to_string())
        }
    }
}

pub(crate) fn decr(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::decr");
    if args.len() != 2 {
        return wrong_args("DECR");
    }
    match store.incr_by(&args[1], -1) {
        Ok(value) => RespFrame::Integer(value),
        Err(StringIntOpError::WrongType) => wrong_type(),
        Err(StringIntOpError::InvalidInteger) => int_error(),
        Err(StringIntOpError::Overflow) => {
            RespFrame::Error("ERR increment or decrement would overflow".to_string())
        }
    }
}

pub(crate) fn incrbyfloat(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::incrbyfloat");
    if args.len() != 3 {
        return wrong_args("INCRBYFLOAT");
    }
    let delta = match parse_f64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.incr_by_float(&args[1], delta) {
        Ok(value) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(value)))),
        Err(_) => match store.value_kind(&args[1]) {
            Some(kind) if kind != "string" => wrong_type(),
            _ => RespFrame::Error("ERR value is not a valid float".to_string()),
        },
    }
}

pub(crate) fn decrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::counter::decrby");
    if args.len() != 3 {
        return wrong_args("DECRBY");
    }
    let delta = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let Some(delta) = delta.checked_neg() else {
        return RespFrame::Error("ERR increment or decrement would overflow".to_string());
    };

    match store.incr_by(&args[1], delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(StringIntOpError::WrongType) => wrong_type(),
        Err(StringIntOpError::InvalidInteger) => int_error(),
        Err(StringIntOpError::Overflow) => {
            RespFrame::Error("ERR increment or decrement would overflow".to_string())
        }
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

fn parse_f64(raw: &[u8]) -> Result<f64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| RespFrame::Error("ERR value is not a valid float".to_string())),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not a valid float".to_string(),
        )),
    }
}

use crate::util::{Args, f64_to_bytes, int_error, wrong_args, wrong_type};
use engine::store::{HashFloatOpError, HashIntOpError, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn hincrby(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::hash::counter::hincrby");
    if args.len() != 4 {
        return wrong_args("HINCRBY");
    }

    let delta = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.hincrby(&args[1], &args[2], delta) {
        Ok(value) => RespFrame::Integer(value),
        Err(HashIntOpError::WrongType) => wrong_type(),
        Err(HashIntOpError::InvalidInteger) => {
            RespFrame::Error("ERR hash value is not an integer".to_string())
        }
        Err(HashIntOpError::Overflow) => {
            RespFrame::Error("ERR increment or decrement would overflow".to_string())
        }
    }
}

pub(crate) fn hincrbyfloat(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::hash::counter::hincrbyfloat");
    if args.len() != 4 {
        return wrong_args("HINCRBYFLOAT");
    }

    let delta = match parse_f64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.hincrbyfloat(&args[1], &args[2], delta) {
        Ok(value) => RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(value)))),
        Err(HashFloatOpError::WrongType) => wrong_type(),
        Err(HashFloatOpError::InvalidFloat) => {
            RespFrame::Error("ERR hash value is not a float".to_string())
        }
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::hash::counter::parse_i64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_f64(raw: &[u8]) -> Result<f64, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::hash::counter::parse_f64");
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<f64>()
            .map_err(|_| RespFrame::Error("ERR value is not a valid float".to_string())),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not a valid float".to_string(),
        )),
    }
}

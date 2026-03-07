use crate::util::{eq_ascii, int_error, parse_u64_bytes, wrong_args, wrong_type, Args};
use engine::store::{ListSide, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn lmove(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::moves::lmove");
    if args.len() != 5 {
        return wrong_args("LMOVE");
    }

    let from = match parse_side(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let to = match parse_side(&args[4]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.lmove(&args[1], &args[2], from, to) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn brpoplpush(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::moves::brpoplpush");
    if args.len() != 4 {
        return wrong_args("BRPOPLPUSH");
    }
    if parse_timeout(&args[3]).is_err() {
        return int_error();
    }

    match store.rpoplpush(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lmpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::moves::lmpop");
    if args.len() < 4 {
        return wrong_args("LMPOP");
    }
    let num_keys = match parse_usize(&args[1]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 2 + num_keys + 1 {
        return crate::util::syntax_error();
    }

    let keys_end = 2 + num_keys;
    let side = match parse_side(&args[keys_end]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut count = 1usize;
    if args.len() > keys_end + 1 {
        if args.len() != keys_end + 3 || !eq_ascii(&args[keys_end + 1], b"COUNT") {
            return crate::util::syntax_error();
        }
        count = match parse_usize(&args[keys_end + 2]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.lmpop(&args[2..keys_end], side, count) {
        Ok(Some((key, values))) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::Arg(key))),
            RespFrame::Array(Some(
                values
                    .into_iter()
                    .map(|value| RespFrame::Bulk(Some(BulkData::Value(value))))
                    .collect(),
            )),
        ])),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

fn parse_side(raw: &[u8]) -> Result<ListSide, RespFrame> {
    let _trace = profiler::scope("commands::list::moves::parse_side");
    if eq_ascii(raw, b"LEFT") {
        Ok(ListSide::Left)
    } else if eq_ascii(raw, b"RIGHT") {
        Ok(ListSide::Right)
    } else {
        Err(crate::util::syntax_error())
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

fn parse_timeout(raw: &[u8]) -> Result<f64, ()> {
    let _trace = profiler::scope("commands::list::moves::parse_timeout");
    std::str::from_utf8(raw)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .ok_or(())
}

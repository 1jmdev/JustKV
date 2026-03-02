use crate::util::{Args, eq_ascii, int_error, wrong_args, wrong_type};
use engine::store::{ListSide, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn blpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::blocking::blpop");
    block_pop(store, args, ListSide::Left)
}

pub(crate) fn brpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::blocking::brpop");
    block_pop(store, args, ListSide::Right)
}

fn block_pop(store: &Store, args: &Args, side: ListSide) -> RespFrame {
    let _trace = profiler::scope("commands::list::blocking::block_pop");
    if args.len() < 3 {
        return wrong_args(if matches!(side, ListSide::Left) {
            "BLPOP"
        } else {
            "BRPOP"
        });
    }
    if parse_timeout(&args[args.len() - 1]).is_err() {
        return int_error();
    }
    let keys = &args[1..args.len() - 1];
    match store.list_pop_first(keys, side) {
        Ok(Some((key, value))) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::Arg(key))),
            RespFrame::Bulk(Some(BulkData::Value(value))),
        ])),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn blmpop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::blocking::blmpop");
    if args.len() < 5 {
        return wrong_args("BLMPOP");
    }
    if parse_timeout(&args[1]).is_err() {
        return int_error();
    }
    let num_keys = match parse_usize(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 3 + num_keys + 1 {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let keys_start = 3;
    let keys_end = keys_start + num_keys;
    let side = match parse_side(&args[keys_end]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut count = 1usize;
    if args.len() > keys_end + 1 {
        if args.len() != keys_end + 3 || !eq_ascii(&args[keys_end + 1], b"COUNT") {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        count = match parse_usize(&args[keys_end + 2]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.lmpop(&args[keys_start..keys_end], side, count) {
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
    let _trace = profiler::scope("commands::list::blocking::parse_side");
    if eq_ascii(raw, b"LEFT") {
        Ok(ListSide::Left)
    } else if eq_ascii(raw, b"RIGHT") {
        Ok(ListSide::Right)
    } else {
        Err(RespFrame::Error("ERR syntax error".to_string()))
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("commands::list::blocking::parse_usize");
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

fn parse_timeout(raw: &[u8]) -> Result<f64, ()> {
    let _trace = profiler::scope("commands::list::blocking::parse_timeout");
    std::str::from_utf8(raw)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .ok_or(())
}

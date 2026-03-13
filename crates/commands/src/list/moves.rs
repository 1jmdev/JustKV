use crate::list::{lmpop_response, parse_side, parse_timeout, parse_usize};
use crate::util::{Args, eq_ascii, timeout_error, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn lmove(store: &Store, args: &Args) -> RespFrame {
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
    if args.len() != 4 {
        return wrong_args("BRPOPLPUSH");
    }
    if parse_timeout(&args[3]).is_err() {
        return timeout_error();
    }

    match store.rpoplpush(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn rpoplpush(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("RPOPLPUSH");
    }

    match store.rpoplpush(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lmpop(store: &Store, args: &Args) -> RespFrame {
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
        Ok(Some((key, values))) => lmpop_response(key, values),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

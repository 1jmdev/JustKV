use std::time::Duration;

use crate::commands::util::{Args, eq_ascii, wrong_args, wrong_type};
use crate::engine::store::Store;
use crate::engine::value::CompactArg;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn get(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GET");
    }
    match store.get(&args[1]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn set(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SET");
    }

    let mut ttl = None;
    let mut nx = false;
    let mut xx = false;
    let mut return_old = false;
    let mut index = 3;

    while index < args.len() {
        let opt = args[index].as_slice();
        if eq_ascii(opt, b"NX") {
            nx = true;
        } else if eq_ascii(opt, b"XX") {
            xx = true;
        } else if eq_ascii(opt, b"GET") {
            return_old = true;
        } else if eq_ascii(opt, b"EX") {
            index += 1;
            if index >= args.len() {
                return wrong_args("SET");
            }
            let seconds = match parse_u64(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            ttl = Some(Duration::from_secs(seconds));
        } else if eq_ascii(opt, b"PX") {
            index += 1;
            if index >= args.len() {
                return wrong_args("SET");
            }
            let millis = match parse_u64(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            ttl = Some(Duration::from_millis(millis));
        } else {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        index += 1;
    }

    if nx && xx {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let key = args[1].as_slice();
    let value = args[2].as_slice();

    let old_value = if return_old {
        match store.get(key) {
            Ok(value) => value.map(BulkData::Value),
            Err(_) => return wrong_type(),
        }
    } else {
        None
    };
    let success = if nx {
        store.setnx(key, value, ttl)
    } else if xx {
        store.setxx(key, value, ttl)
    } else {
        store.set(key, value, ttl);
        true
    };

    if !success {
        return RespFrame::Bulk(None);
    }

    if return_old {
        RespFrame::Bulk(old_value)
    } else {
        RespFrame::ok()
    }
}

pub(crate) fn setnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("SETNX");
    }
    RespFrame::Integer(store.setnx(&args[1], &args[2], None) as i64)
}

pub(crate) fn getset(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("GETSET");
    }
    match store.getset(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(|value| BulkData::Arg(CompactArg::from_vec(value)))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn getdel(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GETDEL");
    }
    match store.getdel(&args[1]) {
        Ok(value) => RespFrame::Bulk(value.map(|value| BulkData::Arg(CompactArg::from_vec(value)))),
        Err(_) => wrong_type(),
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            RespFrame::Error("ERR value is not an integer or out of range".to_string())
        }),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not an integer or out of range".to_string(),
        )),
    }
}

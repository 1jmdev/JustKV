use std::time::Duration;

use crate::commands::util::{eq_ascii, wrong_args, Args};
use crate::engine::store::Store;
use crate::engine::value::CompactArg;
use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"GET") {
        return Some(get(store, args));
    }
    if eq_ascii(command, b"SET") {
        return Some(set(store, args));
    }
    if eq_ascii(command, b"SETNX") {
        return Some(setnx(store, args));
    }
    if eq_ascii(command, b"GETSET") {
        return Some(getset(store, args));
    }
    if eq_ascii(command, b"GETDEL") {
        return Some(getdel(store, args));
    }
    None
}

fn get(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GET");
    }
    RespFrame::Bulk(store.get(&args[1]).map(BulkData::Value))
}

fn set(store: &Store, args: &Args) -> RespFrame {
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
        store.get(key).map(BulkData::Value)
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

fn setnx(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("SETNX");
    }
    RespFrame::Integer(store.setnx(&args[1], &args[2], None) as i64)
}

fn getset(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("GETSET");
    }
    RespFrame::Bulk(
        store
            .getset(&args[1], &args[2])
            .map(|value| BulkData::Arg(CompactArg::from_vec(value))),
    )
}

fn getdel(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GETDEL");
    }
    RespFrame::Bulk(
        store
            .getdel(&args[1])
            .map(|value| BulkData::Arg(CompactArg::from_vec(value))),
    )
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

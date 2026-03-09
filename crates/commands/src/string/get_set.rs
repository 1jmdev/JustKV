use std::time::Duration;

use crate::util::{parse_u64_bytes, wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

pub(crate) fn get(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::get");
    if args.len() != 2 {
        return wrong_args("GET");
    }
    match store.get(&args[1]) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn set(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::get_set::set");
    if args.len() < 3 {
        return wrong_args("SET");
    }

    let mut ttl = None;
    let mut nx = false;
    let mut xx = false;
    let mut return_old = false;
    let mut index = 3;

    while index < args.len() {
        let option = args[index].as_slice();
        if option.eq_ignore_ascii_case(b"NX") {
            nx = true;
        } else if option.eq_ignore_ascii_case(b"XX") {
            xx = true;
        } else if option.eq_ignore_ascii_case(b"GET") {
            return_old = true;
        } else if option.eq_ignore_ascii_case(b"EX") || option.eq_ignore_ascii_case(b"PX") {
            let use_millis = option.eq_ignore_ascii_case(b"PX");
            index += 1;
            if index >= args.len() {
                return wrong_args("SET");
            }
            let value = match parse_u64(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            ttl = Some(if use_millis {
                Duration::from_millis(value)
            } else {
                Duration::from_secs(value)
            });
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    if nx && xx {
        return crate::util::syntax_error();
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
    let _trace = profiler::scope("commands::string::get_set::setnx");
    if args.len() != 3 {
        return wrong_args("SETNX");
    }
    RespFrame::Integer(store.setnx(&args[1], &args[2], None) as i64)
}

pub(crate) fn getset(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::get_set::getset");
    if args.len() != 3 {
        return wrong_args("GETSET");
    }
    match store.getset(&args[1], &args[2]) {
        Ok(value) => RespFrame::Bulk(value.map(|value| BulkData::Arg(CompactArg::from_vec(value)))),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn getdel(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::string::get_set::getdel");
    if args.len() != 2 {
        return wrong_args("GETDEL");
    }
    match store.getdel(&args[1]) {
        Ok(value) => RespFrame::Bulk(value.map(|value| BulkData::Arg(CompactArg::from_vec(value)))),
        Err(_) => wrong_type(),
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    parse_u64_bytes(raw)
        .ok_or_else(|| RespFrame::error_static("ERR value is not an integer or out of range"))
}

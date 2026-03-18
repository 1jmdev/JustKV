use std::time::Duration;

use crate::util::{Args, parse_u64_bytes, preencode_bulk_slice, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::RespFrame;

#[derive(Clone, Copy)]
enum SetExistenceCondition {
    Any,
    Missing,
    Present,
}

pub(crate) fn get(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GET");
    }
    match store.get_preencoded(&args[1]) {
        Ok(value) => RespFrame::PreEncoded(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn set(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("SET");
    }

    let mut ttl = None;
    let mut condition = SetExistenceCondition::Any;
    let mut return_old = false;
    let mut index = 3;

    while index < args.len() {
        let Some(option) = classify_set_option(args[index].as_slice()) else {
            return crate::util::syntax_error();
        };
        match option {
            SetOption::Nx => {
                if matches!(condition, SetExistenceCondition::Present) {
                    return crate::util::syntax_error();
                }
                condition = SetExistenceCondition::Missing;
            }
            SetOption::Xx => {
                if matches!(condition, SetExistenceCondition::Missing) {
                    return crate::util::syntax_error();
                }
                condition = SetExistenceCondition::Present;
            }
            SetOption::Get => {
                return_old = true;
            }
            SetOption::Ex | SetOption::Px => {
                let use_millis = matches!(option, SetOption::Px);
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
            }
        }
        index += 1;
    }

    let key = args[1].as_slice();
    let value = args[2].as_slice();

    let must_exist = match condition {
        SetExistenceCondition::Any => None,
        SetExistenceCondition::Missing => Some(false),
        SetExistenceCondition::Present => Some(true),
    };
    let (success, old_value) = match store.set_with_options(key, value, ttl, must_exist, return_old)
    {
        Ok(result) => result,
        Err(_) => return wrong_type(),
    };

    if !success {
        return RespFrame::Bulk(None);
    }

    if return_old {
        match old_value {
            Some(value) => RespFrame::PreEncoded(preencode_bulk_slice(value.as_slice())),
            None => RespFrame::Bulk(None),
        }
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
        Ok(Some(value)) => RespFrame::PreEncoded(preencode_bulk_slice(value.as_slice())),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn getdel(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("GETDEL");
    }
    match store.getdel(&args[1]) {
        Ok(Some(value)) => RespFrame::PreEncoded(preencode_bulk_slice(value.as_slice())),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

#[derive(Clone, Copy)]
enum SetOption {
    Ex,
    Get,
    Nx,
    Px,
    Xx,
}

fn classify_set_option(value: &[u8]) -> Option<SetOption> {
    match value {
        [a, b] => match ((*a) | 0x20, (*b) | 0x20) {
            (b'e', b'x') => Some(SetOption::Ex),
            (b'n', b'x') => Some(SetOption::Nx),
            (b'p', b'x') => Some(SetOption::Px),
            (b'x', b'x') => Some(SetOption::Xx),
            _ => None,
        },
        [a, b, c] if ((*a) | 0x20, (*b) | 0x20, (*c) | 0x20) == (b'g', b'e', b't') => {
            Some(SetOption::Get)
        }
        _ => None,
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    parse_u64_bytes(raw)
        .ok_or_else(|| RespFrame::error_static("ERR value is not an integer or out of range"))
}

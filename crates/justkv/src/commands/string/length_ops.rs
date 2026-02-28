use crate::commands::util::{eq_ascii, int_error, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"APPEND") {
        return Some(append(store, args));
    }
    if eq_ascii(command, b"STRLEN") {
        return Some(strlen(store, args));
    }
    if eq_ascii(command, b"SETRANGE") {
        return Some(setrange(store, args));
    }
    if eq_ascii(command, b"GETRANGE") {
        return Some(getrange(store, args));
    }
    None
}

fn append(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("APPEND");
    }
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
    }
    RespFrame::Integer(store.append(&args[1], &args[2]) as i64)
}

fn strlen(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("STRLEN");
    }
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
    }
    RespFrame::Integer(store.strlen(&args[1]) as i64)
}

fn setrange(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("SETRANGE");
    }
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
    }

    let offset = match parse_usize(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    RespFrame::Integer(store.setrange(&args[1], offset, &args[3]) as i64)
}

fn getrange(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("GETRANGE");
    }
    if matches!(store.value_kind(&args[1]), Some("hash")) {
        return wrong_type();
    }

    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let end = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    RespFrame::Bulk(Some(BulkData::from_vec(
        store.getrange(&args[1], start, end),
    )))
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error())?,
        Err(_) => return Err(int_error()),
    };
    usize::try_from(value).map_err(|_| RespFrame::Error("ERR offset is out of range".to_string()))
}

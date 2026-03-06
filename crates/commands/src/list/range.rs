use crate::util::{eq_ascii, int_error, wrong_args, wrong_type, Args};
use engine::store::{ListInsertPosition, ListSetError, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn lindex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::lindex");
    if args.len() != 3 {
        return wrong_args("LINDEX");
    }
    let index = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.lindex(&args[1], index) {
        Ok(value) => RespFrame::Bulk(value.map(BulkData::Value)),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lrange(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::lrange");
    if args.len() != 4 {
        return wrong_args("LRANGE");
    }
    let start = match parse_i64(args[2].slice()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let stop = match parse_i64(args[3].slice()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.lrange_encode(args[1].slice(), start, stop) {
        Ok(bytes) => RespFrame::PreEncoded(bytes),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lset(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::lset");
    if args.len() != 4 {
        return wrong_args("LSET");
    }
    let index = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.lset(&args[1], index, &args[3]) {
        Ok(()) => RespFrame::ok(),
        Err(ListSetError::NoSuchKey) => RespFrame::Error("ERR no such key".to_string()),
        Err(ListSetError::OutOfRange) => RespFrame::Error("ERR index out of range".to_string()),
        Err(ListSetError::WrongType) => wrong_type(),
    }
}

pub(crate) fn ltrim(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::ltrim");
    if args.len() != 4 {
        return wrong_args("LTRIM");
    }
    let start = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let stop = match parse_i64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match store.ltrim(&args[1], start, stop) {
        Ok(()) => RespFrame::ok(),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn linsert(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::linsert");
    if args.len() != 5 {
        return wrong_args("LINSERT");
    }
    let position = if eq_ascii(&args[2], b"BEFORE") {
        ListInsertPosition::Before
    } else if eq_ascii(&args[2], b"AFTER") {
        ListInsertPosition::After
    } else {
        return RespFrame::Error("ERR syntax error".to_string());
    };
    match store.linsert(&args[1], position, &args[3], &args[4]) {
        Ok(result) => RespFrame::Integer(result),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn lpos(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::list::range::lpos");
    if args.len() < 3 {
        return wrong_args("LPOS");
    }

    let mut rank = 1_i64;
    let mut count = None;
    let mut maxlen = None;
    let mut index = 3;
    while index < args.len() {
        if eq_ascii(&args[index], b"RANK") {
            index += 1;
            if index >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            rank = match parse_i64(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            if rank == 0 {
                return RespFrame::Error("ERR RANK can't be zero: use 1 to start from the first match, 2 from the second ... or use negative to start from the end of the list".to_string());
            }
        } else if eq_ascii(&args[index], b"COUNT") {
            index += 1;
            if index >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            count = Some(match parse_usize(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            });
        } else if eq_ascii(&args[index], b"MAXLEN") {
            index += 1;
            if index >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            maxlen = Some(match parse_usize(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            });
        } else {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        index += 1;
    }

    match store.lpos(&args[1], &args[2], rank, count, maxlen) {
        Ok(Some(positions)) if count.is_some() => RespFrame::Array(Some(
            positions.into_iter().map(RespFrame::Integer).collect(),
        )),
        Ok(Some(mut positions)) => RespFrame::Integer(positions.remove(0)),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    let _trace = profiler::scope("commands::list::range::parse_i64");
    if raw.is_empty() {
        return Err(int_error());
    }

    let mut index = 0;
    let mut negative = false;
    match raw[0] {
        b'-' => {
            negative = true;
            index = 1;
        }
        b'+' => index = 1,
        _ => {}
    }

    if index == raw.len() {
        return Err(int_error());
    }

    let mut value: i64 = 0;
    while index < raw.len() {
        let digit = raw[index].wrapping_sub(b'0');
        if digit > 9 {
            return Err(int_error());
        }
        value = value
            .checked_mul(10)
            .and_then(|n| n.checked_add(i64::from(digit)))
            .ok_or_else(int_error)?;
        index += 1;
    }

    if negative {
        value.checked_neg().ok_or_else(int_error)
    } else {
        Ok(value)
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("commands::list::range::parse_usize");
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

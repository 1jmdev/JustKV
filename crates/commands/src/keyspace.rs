use crate::util::{
    eq_ascii, int_error, parse_i64_bytes, parse_u64_bytes, u64_to_bytes, wrong_args, wrong_type,
    Args,
};
use engine::store::{RestoreError, SortError, SortOptions, SortOrder, SortResult, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn del(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::del");
    if args.len() < 2 {
        return wrong_args("DEL");
    }
    RespFrame::Integer(store.del(&args[1..]))
}

pub(crate) fn exists(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::exists");
    if args.len() < 2 {
        return wrong_args("EXISTS");
    }
    RespFrame::Integer(store.exists(&args[1..]))
}

pub(crate) fn touch(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::touch");
    if args.len() < 2 {
        return wrong_args("TOUCH");
    }
    RespFrame::Integer(store.touch(&args[1..]))
}

pub(crate) fn unlink(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::unlink");
    if args.len() < 2 {
        return wrong_args("UNLINK");
    }
    RespFrame::Integer(store.unlink(&args[1..]))
}

pub(crate) fn key_type(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::key_type");
    if args.len() != 2 {
        return wrong_args("TYPE");
    }
    RespFrame::Simple(store.key_type(&args[1]).to_string())
}

pub(crate) fn rename(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::rename");
    if args.len() != 3 {
        return wrong_args("RENAME");
    }
    if store.rename(&args[1], &args[2]) {
        RespFrame::ok()
    } else {
        RespFrame::Error("ERR no such key".to_string())
    }
}

pub(crate) fn renamenx(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::renamenx");
    if args.len() != 3 {
        return wrong_args("RENAMENX");
    }
    match store.renamenx(&args[1], &args[2]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR no such key".to_string()),
    }
}

pub(crate) fn dbsize(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::dbsize");
    if args.len() != 1 {
        return wrong_args("DBSIZE");
    }
    RespFrame::Integer(store.dbsize())
}

pub(crate) fn keys(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::keys");
    if args.len() != 2 {
        return wrong_args("KEYS");
    }

    RespFrame::Array(Some(
        store
            .keys(&args[1])
            .into_iter()
            .map(|key| RespFrame::Bulk(Some(BulkData::from_vec(key))))
            .collect(),
    ))
}

pub(crate) fn scan(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::scan");
    if args.len() < 2 {
        return wrong_args("SCAN");
    }

    let cursor = match parse_u64(&args[1]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut pattern = None;
    let mut count = 10usize;
    let mut value_type = None;
    let mut index = 2;
    while index < args.len() {
        if eq_ascii(&args[index], b"MATCH") {
            index += 1;
            if index >= args.len() {
                return crate::util::syntax_error();
            }
            pattern = Some(args[index].as_slice());
        } else if eq_ascii(&args[index], b"COUNT") {
            index += 1;
            if index >= args.len() {
                return crate::util::syntax_error();
            }
            count = match parse_usize(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
        } else if eq_ascii(&args[index], b"TYPE") {
            index += 1;
            if index >= args.len() {
                return crate::util::syntax_error();
            }
            value_type = Some(args[index].as_slice());
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    let (next, keys) = store.scan(cursor, pattern, count, value_type);
    RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::from_vec(u64_to_bytes(next)))),
        RespFrame::Array(Some(
            keys.into_iter()
                .map(|key| RespFrame::Bulk(Some(BulkData::Arg(key))))
                .collect(),
        )),
    ]))
}

pub(crate) fn move_key(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::move_key");
    if args.len() != 3 {
        return wrong_args("MOVE");
    }

    let db = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.move_key(&args[1], db) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR DB index is out of range".to_string()),
    }
}

pub(crate) fn dump(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::dump");
    if args.len() != 2 {
        return wrong_args("DUMP");
    }
    RespFrame::Bulk(store.dump(&args[1]).map(BulkData::from_vec))
}

pub(crate) fn restore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::restore");
    if args.len() < 4 {
        return wrong_args("RESTORE");
    }

    let ttl_ms = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut replace = false;
    let mut index = 4;
    while index < args.len() {
        if eq_ascii(&args[index], b"REPLACE") {
            replace = true;
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    match store.restore(&args[1], ttl_ms, &args[3], replace) {
        Ok(()) => RespFrame::ok(),
        Err(RestoreError::BusyKey) => {
            RespFrame::Error("BUSYKEY Target key name already exists.".to_string())
        }
        Err(RestoreError::InvalidPayload) => {
            RespFrame::Error("ERR DUMP payload version or checksum are wrong".to_string())
        }
    }
}

pub(crate) fn sort(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::sort");
    if args.len() < 2 {
        return wrong_args("SORT");
    }

    let mut options = SortOptions {
        alpha: false,
        order: SortOrder::Asc,
        limit: None,
        store: None,
    };

    let mut index = 2;
    while index < args.len() {
        if eq_ascii(&args[index], b"ASC") {
            options.order = SortOrder::Asc;
        } else if eq_ascii(&args[index], b"DESC") {
            options.order = SortOrder::Desc;
        } else if eq_ascii(&args[index], b"ALPHA") {
            options.alpha = true;
        } else if eq_ascii(&args[index], b"LIMIT") {
            if index + 2 >= args.len() {
                return crate::util::syntax_error();
            }
            let offset = match parse_usize(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let count = match parse_usize(&args[index + 2]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            options.limit = Some((offset, count));
            index += 2;
        } else if eq_ascii(&args[index], b"STORE") {
            if index + 1 >= args.len() {
                return crate::util::syntax_error();
            }
            options.store = Some(args[index + 1].to_vec());
            index += 1;
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    match store.sort(&args[1], &options) {
        Ok(SortResult::Values(values)) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(Some(BulkData::from_vec(value))))
                .collect(),
        )),
        Ok(SortResult::Stored(size)) => RespFrame::Integer(size),
        Err(SortError::WrongType) => wrong_type(),
        Err(SortError::InvalidNumber) => {
            RespFrame::Error("ERR One or more scores can't be converted into double".to_string())
        }
    }
}

pub(crate) fn copy(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::copy");
    if args.len() < 3 {
        return wrong_args("COPY");
    }
    if args[1].as_slice() == args[2].as_slice() {
        return RespFrame::Error("ERR source and destination objects are the same".to_string());
    }

    let mut replace = false;
    let mut db = 0i64;
    let mut index = 3;
    while index < args.len() {
        if eq_ascii(&args[index], b"REPLACE") {
            replace = true;
        } else if eq_ascii(&args[index], b"DB") {
            if index + 1 >= args.len() {
                return crate::util::syntax_error();
            }
            db = match parse_i64(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            index += 1;
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    if db != 0 {
        return RespFrame::Error("ERR DB index is out of range".to_string());
    }

    RespFrame::Integer(store.copy(&args[1], &args[2], replace))
}

pub(crate) fn flushdb(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::flushdb");
    if args.len() != 1 {
        return wrong_args("FLUSHDB");
    }
    let _ = store.flushdb();
    RespFrame::ok()
}

pub(crate) fn flushall(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::keyspace::flushall");
    if args.len() != 1 {
        return wrong_args("FLUSHALL");
    }
    let _ = store.flushdb();
    RespFrame::ok()
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    parse_u64_bytes(raw).ok_or_else(int_error)
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_u64(raw)?;
    usize::try_from(value).map_err(|_| int_error())
}

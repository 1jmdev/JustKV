use crate::stream::parse::{parse_count, parse_stream_id};
use crate::util::{Args, wrong_args, wrong_type};
use engine::store::{Store, StreamWriteError, XDelexPolicy};
use protocol::types::RespFrame;
use types::value::StreamId;

fn stream_write_error_response(error: StreamWriteError) -> RespFrame {
    match error {
        StreamWriteError::WrongType => wrong_type(),
        StreamWriteError::InternalInvariant => {
            RespFrame::ErrorStatic("ERR internal stream state inconsistency")
        }
    }
}

pub(crate) fn xdel(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::delete::xdel");
    if args.len() < 3 {
        return wrong_args("XDEL");
    }
    let mut ids = Vec::with_capacity(args.len() - 2);
    for raw in &args[2..] {
        match parse_stream_id(raw) {
            Ok(id) => ids.push(id),
            Err(response) => return response,
        }
    }
    match store.xdel(&args[1], &ids) {
        Ok(value) => RespFrame::Integer(value),
        Err(error) => stream_write_error_response(error),
    }
}

pub(crate) fn xdelex(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::delete::xdelex");
    if args.len() < 5 {
        return wrong_args("XDELEX");
    }

    let mut policy = XDelexPolicy::KeepRef;
    let mut saw_policy = false;
    let mut ids: Option<Vec<StreamId>> = None;
    let mut index = 2usize;
    while index < args.len() {
        if args[index].eq_ignore_ascii_case(b"KEEPREF") {
            if saw_policy {
                return crate::util::syntax_error();
            }
            policy = XDelexPolicy::KeepRef;
            saw_policy = true;
            index += 1;
            continue;
        }
        if args[index].eq_ignore_ascii_case(b"DELREF") {
            if saw_policy {
                return crate::util::syntax_error();
            }
            policy = XDelexPolicy::DelRef;
            saw_policy = true;
            index += 1;
            continue;
        }
        if args[index].eq_ignore_ascii_case(b"ACKED") {
            if saw_policy {
                return crate::util::syntax_error();
            }
            policy = XDelexPolicy::Acked;
            saw_policy = true;
            index += 1;
            continue;
        }
        if args[index].eq_ignore_ascii_case(b"IDS") {
            if ids.is_some() || index + 1 >= args.len() {
                return crate::util::syntax_error();
            }
            let count = match parse_count(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let start = index + 2;
            let end = start + count;
            if end > args.len() {
                return wrong_args("XDELEX");
            }
            let mut parsed_ids = Vec::with_capacity(count);
            for raw in &args[start..end] {
                match parse_stream_id(raw) {
                    Ok(id) => parsed_ids.push(id),
                    Err(response) => return response,
                }
            }
            ids = Some(parsed_ids);
            index = end;
            continue;
        }
        return crate::util::syntax_error();
    }

    let Some(ids) = ids else {
        return crate::util::syntax_error();
    };

    match store.xdelex(&args[1], policy, &ids) {
        Ok(values) => RespFrame::Array(Some(values.into_iter().map(RespFrame::Integer).collect())),
        Err(error) => stream_write_error_response(error),
    }
}

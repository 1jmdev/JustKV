use crate::stream::parse::{parse_stream_id, parse_xadd_id, parse_xtrim_args, stream_id_to_bulk};
use crate::util::{Args, wrong_args, wrong_type};
use engine::store::{Store, XTrimMode};
use protocol::types::RespFrame;
use types::value::StreamId;

pub(crate) fn xadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::add::xadd");
    if args.len() < 4 {
        return wrong_args("XADD");
    }

    let mut index = 2;
    let mut nomkstream = false;
    if args[index].eq_ignore_ascii_case(b"NOMKSTREAM") {
        nomkstream = true;
        index += 1;
    }

    let (trim, next_index) = match parse_xtrim_args(args, index) {
        Ok(value) => value,
        Err(response) => return response,
    };
    index = next_index;

    if index >= args.len() {
        return crate::util::syntax_error();
    }
    let id = match parse_xadd_id(&args[index]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    index += 1;

    if index >= args.len() || (args.len() - index) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'xadd' command".to_string());
    }
    let fields: Vec<_> = args[index..]
        .chunks(2)
        .map(|chunk| (chunk[0].clone(), chunk[1].clone()))
        .collect();

    match store.xadd(&args[1], id, &fields, trim, nomkstream) {
        Ok(Some(id)) => stream_id_to_bulk(id),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xlen(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::add::xlen");
    if args.len() != 2 {
        return wrong_args("XLEN");
    }
    match store.xlen(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xdel(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::add::xdel");
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
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xtrim(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::add::xtrim");
    if args.len() < 4 {
        return wrong_args("XTRIM");
    }
    let mode = if args[2].eq_ignore_ascii_case(b"MAXLEN") {
        XTrimMode::MaxLen
    } else if args[2].eq_ignore_ascii_case(b"MINID") {
        XTrimMode::MinId
    } else {
        return crate::util::syntax_error();
    };

    let mut index = 3;
    if args[index].as_slice() == b"~" || args[index].as_slice() == b"=" {
        index += 1;
    }
    if index >= args.len() {
        return crate::util::syntax_error();
    }

    let threshold = if matches!(mode, XTrimMode::MaxLen) {
        let count = match super::parse::parse_count(&args[index]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        StreamId {
            ms: count as u64,
            seq: 0,
        }
    } else {
        match parse_stream_id(&args[index]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    };
    index += 1;

    let mut limit = None;
    if index + 1 < args.len() && args[index].eq_ignore_ascii_case(b"LIMIT") {
        limit = match super::parse::parse_count(&args[index + 1]) {
            Ok(value) => Some(value),
            Err(response) => return response,
        };
    }

    match store.xtrim(&args[1], mode, threshold, limit) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

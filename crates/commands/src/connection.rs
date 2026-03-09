use crate::util::{eq_ascii, parse_i64_bytes, parse_u64_bytes, wrong_args, Args};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn auth(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::auth");
    if args.len() != 2 && args.len() != 3 {
        return wrong_args("AUTH");
    }
    RespFrame::ok()
}

pub(crate) fn hello(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::hello");
    if args.len() == 1 {
        return hello_response(2);
    }

    let Some(version) = parse_u64_bytes(&args[1]) else {
        return RespFrame::error_static("ERR Protocol version is not an integer or out of range");
    };

    match version {
        2 | 3 => hello_response(version as u8),
        _ => RespFrame::error_static("NOPROTO unsupported protocol version"),
    }
}

fn hello_response(proto: u8) -> RespFrame {
    let _trace = profiler::scope("commands::connection::hello_response");
    if proto == 3 {
        return RespFrame::Map(vec![
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"server".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(b"betterkv".to_vec()))),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"version".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(
                    env!("CARGO_PKG_VERSION").as_bytes().to_vec(),
                ))),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"proto".to_vec()))),
                RespFrame::Integer(3),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"id".to_vec()))),
                RespFrame::Integer(1),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"mode".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(b"standalone".to_vec()))),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"role".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(b"master".to_vec()))),
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"modules".to_vec()))),
                RespFrame::Array(Some(vec![])),
            ),
        ]);
    }

    RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::from_vec(b"server".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"betterkv".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"version".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(
            env!("CARGO_PKG_VERSION").as_bytes().to_vec(),
        ))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"proto".to_vec()))),
        RespFrame::Integer(proto as i64),
        RespFrame::Bulk(Some(BulkData::from_vec(b"id".to_vec()))),
        RespFrame::Integer(1),
        RespFrame::Bulk(Some(BulkData::from_vec(b"mode".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"standalone".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"role".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"master".to_vec()))),
        RespFrame::Bulk(Some(BulkData::from_vec(b"modules".to_vec()))),
        RespFrame::Array(Some(vec![])),
    ]))
}

pub(crate) fn client(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::client");
    if args.len() < 2 {
        return wrong_args("CLIENT");
    }

    let sub = args[1].as_slice();
    if eq_ascii(sub, b"SETINFO") || eq_ascii(sub, b"SETNAME") {
        RespFrame::ok()
    } else if eq_ascii(sub, b"GETNAME") {
        RespFrame::Bulk(None)
    } else if eq_ascii(sub, b"ID") {
        RespFrame::Integer(1)
    } else {
        RespFrame::error_static("ERR unknown subcommand for CLIENT")
    }
}

pub(crate) fn select_db(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::select_db");
    if args.len() != 2 {
        return wrong_args("SELECT");
    }

    match parse_i64_bytes(&args[1]) {
        Some(index) if (0..=15).contains(&index) => RespFrame::ok(),
        Some(_) => RespFrame::error_static("ERR DB index is out of range"),
        None => RespFrame::error_static("ERR value is not an integer or out of range"),
    }
}

pub(crate) fn quit(_args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::quit");
    RespFrame::ok()
}

pub(crate) fn ping(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::ping");
    if args.len() == 1 {
        return RespFrame::simple_static("PONG");
    }
    if args.len() == 2 {
        return RespFrame::Bulk(Some(BulkData::Arg(args[1].clone())));
    }
    wrong_args("PING")
}

pub(crate) fn echo(args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::connection::echo");
    if args.len() != 2 {
        return wrong_args("ECHO");
    }
    RespFrame::Bulk(Some(BulkData::Arg(args[1].clone())))
}

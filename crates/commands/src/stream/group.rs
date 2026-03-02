use crate::stream::parse::parse_stream_id;
use crate::util::{Args, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn xgroup(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::stream::group::xgroup");
    if args.len() < 2 {
        return wrong_args("XGROUP");
    }

    if args[1].eq_ignore_ascii_case(b"CREATE") {
        if args.len() < 5 {
            return wrong_args("XGROUP");
        }
        let mkstream = args.len() == 6 && args[5].eq_ignore_ascii_case(b"MKSTREAM");
        let id = match parse_stream_id(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        return match store.xgroup_create(&args[2], &args[3], id, mkstream) {
            Ok(true) => RespFrame::ok(),
            Ok(false) => {
                RespFrame::Error("BUSYGROUP Consumer Group name already exists".to_string())
            }
            Err(_) => wrong_type(),
        };
    }

    if args[1].eq_ignore_ascii_case(b"SETID") {
        if args.len() != 5 {
            return wrong_args("XGROUP");
        }
        let id = match parse_stream_id(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        return match store.xgroup_setid(&args[2], &args[3], id) {
            Ok(true) => RespFrame::ok(),
            Ok(false) => RespFrame::Error("ERR no such key".to_string()),
            Err(_) => wrong_type(),
        };
    }

    if args[1].eq_ignore_ascii_case(b"DESTROY") {
        if args.len() != 4 {
            return wrong_args("XGROUP");
        }
        return match store.xgroup_destroy(&args[2], &args[3]) {
            Ok(value) => RespFrame::Integer(value),
            Err(_) => wrong_type(),
        };
    }

    RespFrame::Error("ERR syntax error".to_string())
}

use crate::commands::util::{wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"DEL" => Some(del(store, args)),
        b"EXISTS" => Some(exists(store, args)),
        b"TOUCH" => Some(touch(store, args)),
        b"TYPE" => Some(key_type(store, args)),
        b"RENAME" => Some(rename(store, args)),
        b"RENAMENX" => Some(renamenx(store, args)),
        b"DBSIZE" => Some(dbsize(store, args)),
        b"KEYS" => Some(keys(store, args)),
        b"FLUSHDB" => Some(flushdb(store, args)),
        _ => None,
    }
}

fn del(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("DEL");
    }
    RespFrame::Integer(store.del(&args[1..]))
}

fn exists(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("EXISTS");
    }
    RespFrame::Integer(store.exists(&args[1..]))
}

fn touch(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 {
        return wrong_args("TOUCH");
    }
    RespFrame::Integer(store.touch(&args[1..]))
}

fn key_type(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("TYPE");
    }
    RespFrame::Simple(store.key_type(&args[1]).to_string())
}

fn rename(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("RENAME");
    }
    if store.rename(&args[1], args[2].clone()) {
        RespFrame::ok()
    } else {
        RespFrame::Error("ERR no such key".to_string())
    }
}

fn renamenx(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("RENAMENX");
    }
    match store.renamenx(&args[1], args[2].clone()) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR no such key".to_string()),
    }
}

fn dbsize(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 1 {
        return wrong_args("DBSIZE");
    }
    RespFrame::Integer(store.dbsize())
}

fn keys(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("KEYS");
    }

    RespFrame::Array(Some(
        store
            .keys(&args[1])
            .into_iter()
            .map(|key| RespFrame::Bulk(Some(key)))
            .collect(),
    ))
}

fn flushdb(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 1 {
        return wrong_args("FLUSHDB");
    }
    let _ = store.flushdb();
    RespFrame::ok()
}

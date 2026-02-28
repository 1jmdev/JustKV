use crate::commands::util::{wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"DEL" => Some(del(store, args)),
        b"EXISTS" => Some(exists(store, args)),
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

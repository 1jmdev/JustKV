use crate::commands::util::{int_error, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"EXPIRE" => Some(expire(store, args)),
        b"TTL" => Some(ttl(store, args)),
        _ => None,
    }
}

fn expire(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("EXPIRE");
    }

    let seconds = match std::str::from_utf8(&args[2]) {
        Ok(value) => match value.parse::<u64>() {
            Ok(value) => value,
            Err(_) => return int_error(),
        },
        Err(_) => return int_error(),
    };

    RespFrame::Integer(store.expire(&args[1], seconds))
}

fn ttl(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("TTL");
    }
    RespFrame::Integer(store.ttl(&args[1]))
}

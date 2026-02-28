use crate::commands::util::{int_error, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    match command {
        b"EXPIRE" => Some(expire(store, args)),
        b"PEXPIRE" => Some(pexpire(store, args)),
        b"EXPIREAT" => Some(expireat(store, args)),
        b"PEXPIREAT" => Some(pexpireat(store, args)),
        b"PERSIST" => Some(persist(store, args)),
        b"TTL" => Some(ttl(store, args)),
        b"PTTL" => Some(pttl(store, args)),
        _ => None,
    }
}

fn expire(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("EXPIRE");
    }

    let seconds = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    RespFrame::Integer(store.expire(&args[1], seconds))
}

fn ttl(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("TTL");
    }
    RespFrame::Integer(store.ttl(&args[1]))
}

fn pexpire(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("PEXPIRE");
    }
    let millis = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.pexpire(&args[1], millis))
}

fn expireat(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("EXPIREAT");
    }
    let sec = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.expire_at(&args[1], sec))
}

fn pexpireat(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("PEXPIREAT");
    }
    let millis = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.pexpire_at(&args[1], millis))
}

fn persist(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("PERSIST");
    }
    RespFrame::Integer(store.persist(&args[1]))
}

fn pttl(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 {
        return wrong_args("PTTL");
    }
    RespFrame::Integer(store.pttl(&args[1]))
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

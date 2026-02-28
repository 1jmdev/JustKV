use crate::commands::util::{eq_ascii, int_error, wrong_args, Args};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if eq_ascii(command, b"EXPIRE") {
        return Some(expire(store, args));
    }
    if eq_ascii(command, b"PEXPIRE") {
        return Some(pexpire(store, args));
    }
    if eq_ascii(command, b"EXPIREAT") {
        return Some(expireat(store, args));
    }
    if eq_ascii(command, b"PEXPIREAT") {
        return Some(pexpireat(store, args));
    }
    if eq_ascii(command, b"PERSIST") {
        return Some(persist(store, args));
    }
    if eq_ascii(command, b"TTL") {
        return Some(ttl(store, args));
    }
    if eq_ascii(command, b"PTTL") {
        return Some(pttl(store, args));
    }
    None
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

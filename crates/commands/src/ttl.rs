use crate::util::{Args, int_error, wrong_args};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn expire(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::expire");
    if args.len() != 3 {
        return wrong_args("EXPIRE");
    }

    let seconds = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    RespFrame::Integer(store.expire(&args[1], seconds))
}

pub(crate) fn ttl(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::ttl");
    if args.len() != 2 {
        return wrong_args("TTL");
    }
    RespFrame::Integer(store.ttl(&args[1]))
}

pub(crate) fn pexpire(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::pexpire");
    if args.len() != 3 {
        return wrong_args("PEXPIRE");
    }
    let millis = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.pexpire(&args[1], millis))
}

pub(crate) fn expireat(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::expireat");
    if args.len() != 3 {
        return wrong_args("EXPIREAT");
    }
    let sec = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.expire_at(&args[1], sec))
}

pub(crate) fn pexpireat(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::pexpireat");
    if args.len() != 3 {
        return wrong_args("PEXPIREAT");
    }
    let millis = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    RespFrame::Integer(store.pexpire_at(&args[1], millis))
}

pub(crate) fn persist(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::persist");
    if args.len() != 2 {
        return wrong_args("PERSIST");
    }
    RespFrame::Integer(store.persist(&args[1]))
}

pub(crate) fn pttl(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::ttl::pttl");
    if args.len() != 2 {
        return wrong_args("PTTL");
    }
    RespFrame::Integer(store.pttl(&args[1]))
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    let _trace = profiler::scope("commands::ttl::parse_u64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

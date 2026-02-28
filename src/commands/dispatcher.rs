use std::time::Duration;

use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn dispatch(store: &Store, frame: RespFrame) -> RespFrame {
    let args = match parse_command(frame) {
        Ok(args) => args,
        Err(err) => return RespFrame::Error(err),
    };

    if args.is_empty() {
        return RespFrame::Error("ERR empty command".to_string());
    }

    let command = ascii_upper(&args[0]);
    match command.as_slice() {
        b"PING" => ping(&args),
        b"ECHO" => echo(&args),
        b"GET" => get(store, &args),
        b"SET" => set(store, &args),
        b"DEL" => del(store, &args),
        b"EXISTS" => exists(store, &args),
        b"INCR" => incr(store, &args),
        b"MGET" => mget(store, &args),
        b"MSET" => mset(store, &args),
        b"EXPIRE" => expire(store, &args),
        b"TTL" => ttl(store, &args),
        _ => RespFrame::Error("ERR unknown command".to_string()),
    }
}

fn parse_command(frame: RespFrame) -> Result<Vec<Vec<u8>>, String> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error".to_string());
    };

    let mut args = Vec::with_capacity(items.len());
    for item in items {
        match item {
            RespFrame::Bulk(Some(bytes)) => args.push(bytes),
            RespFrame::Simple(value) => args.push(value.into_bytes()),
            _ => return Err("ERR invalid argument type".to_string()),
        }
    }

    Ok(args)
}

fn ascii_upper(raw: &[u8]) -> Vec<u8> {
    raw.iter().map(u8::to_ascii_uppercase).collect()
}

fn ping(args: &[Vec<u8>]) -> RespFrame {
    if args.len() == 1 {
        return RespFrame::Simple("PONG".to_string());
    }
    if args.len() == 2 {
        return RespFrame::Bulk(Some(args[1].clone()));
    }
    RespFrame::Error("ERR wrong number of arguments for 'PING' command".to_string())
}

fn echo(args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'ECHO' command".to_string());
    }
    RespFrame::Bulk(Some(args[1].clone()))
}

fn get(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'GET' command".to_string());
    }

    let value = store.get(&args[1]);
    RespFrame::Bulk(value)
}

fn set(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 3 {
        return RespFrame::Error("ERR wrong number of arguments for 'SET' command".to_string());
    }

    store.set(args[1].clone(), args[2].clone(), None::<Duration>);
    RespFrame::ok()
}

fn del(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'DEL' command".to_string());
    }

    let keys = args[1..].to_vec();
    RespFrame::Integer(store.del(&keys))
}

fn exists(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'EXISTS' command".to_string());
    }

    let keys = args[1..].to_vec();
    RespFrame::Integer(store.exists(&keys))
}

fn incr(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'INCR' command".to_string());
    }

    match store.incr(&args[1]) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR value is not an integer or out of range".to_string()),
    }
}

fn mget(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'MGET' command".to_string());
    }

    let keys = args[1..].to_vec();
    let values = store.mget(&keys);

    RespFrame::Array(Some(
        values
            .into_iter()
            .map(RespFrame::Bulk)
            .collect::<Vec<RespFrame>>(),
    ))
}

fn mset(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'MSET' command".to_string());
    }

    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].clone(), chunk[1].clone()));
    }

    store.mset(&pairs);
    RespFrame::ok()
}

fn expire(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 3 {
        return RespFrame::Error("ERR wrong number of arguments for 'EXPIRE' command".to_string());
    }

    let ttl_text = match std::str::from_utf8(&args[2]) {
        Ok(value) => value,
        Err(_) => {
            return RespFrame::Error("ERR value is not an integer or out of range".to_string());
        }
    };

    match ttl_text.parse::<u64>() {
        Ok(seconds) => RespFrame::Integer(store.expire(&args[1], seconds)),
        Err(_) => RespFrame::Error("ERR value is not an integer or out of range".to_string()),
    }
}

fn ttl(store: &Store, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'TTL' command".to_string());
    }

    RespFrame::Integer(store.ttl(&args[1]))
}

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

    let command = args[0].to_ascii_uppercase();
    match command.as_str() {
        "PING" => ping(&args),
        "ECHO" => echo(&args),
        "GET" => get(store, &args),
        "SET" => set(store, &args),
        "DEL" => del(store, &args),
        "EXISTS" => exists(store, &args),
        "INCR" => incr(store, &args),
        "MGET" => mget(store, &args),
        "MSET" => mset(store, &args),
        "EXPIRE" => expire(store, &args),
        "TTL" => ttl(store, &args),
        _ => RespFrame::Error("ERR unknown command".to_string()),
    }
}

fn parse_command(frame: RespFrame) -> Result<Vec<String>, String> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error".to_string());
    };

    let mut args = Vec::with_capacity(items.len());
    for item in items {
        match item {
            RespFrame::Bulk(Some(bytes)) => {
                let arg = String::from_utf8(bytes).map_err(|_| "ERR invalid utf8".to_string())?;
                args.push(arg);
            }
            RespFrame::Simple(value) => args.push(value),
            _ => return Err("ERR invalid argument type".to_string()),
        }
    }

    Ok(args)
}

fn ping(args: &[String]) -> RespFrame {
    if args.len() == 1 {
        return RespFrame::Simple("PONG".to_string());
    }
    if args.len() == 2 {
        return RespFrame::Bulk(Some(args[1].as_bytes().to_vec()));
    }
    RespFrame::Error("ERR wrong number of arguments for 'PING' command".to_string())
}

fn echo(args: &[String]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'ECHO' command".to_string());
    }
    RespFrame::Bulk(Some(args[1].as_bytes().to_vec()))
}

fn get(store: &Store, args: &[String]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'GET' command".to_string());
    }

    let value = store.get(args[1].as_bytes());
    RespFrame::Bulk(value)
}

fn set(store: &Store, args: &[String]) -> RespFrame {
    if args.len() != 3 {
        return RespFrame::Error("ERR wrong number of arguments for 'SET' command".to_string());
    }

    store.set(
        args[1].as_bytes().to_vec(),
        args[2].as_bytes().to_vec(),
        None::<Duration>,
    );
    RespFrame::ok()
}

fn del(store: &Store, args: &[String]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'DEL' command".to_string());
    }

    let keys: Vec<Vec<u8>> = args[1..]
        .iter()
        .map(|part| part.as_bytes().to_vec())
        .collect();
    RespFrame::Integer(store.del(&keys))
}

fn exists(store: &Store, args: &[String]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'EXISTS' command".to_string());
    }

    let keys: Vec<Vec<u8>> = args[1..]
        .iter()
        .map(|part| part.as_bytes().to_vec())
        .collect();
    RespFrame::Integer(store.exists(&keys))
}

fn incr(store: &Store, args: &[String]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'INCR' command".to_string());
    }

    match store.incr(args[1].as_bytes()) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => RespFrame::Error("ERR value is not an integer or out of range".to_string()),
    }
}

fn mget(store: &Store, args: &[String]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'MGET' command".to_string());
    }

    let keys: Vec<Vec<u8>> = args[1..]
        .iter()
        .map(|part| part.as_bytes().to_vec())
        .collect();
    let values = store.mget(&keys);

    RespFrame::Array(Some(
        values
            .into_iter()
            .map(RespFrame::Bulk)
            .collect::<Vec<RespFrame>>(),
    ))
}

fn mset(store: &Store, args: &[String]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'MSET' command".to_string());
    }

    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for chunk in args[1..].chunks(2) {
        pairs.push((chunk[0].as_bytes().to_vec(), chunk[1].as_bytes().to_vec()));
    }

    store.mset(&pairs);
    RespFrame::ok()
}

fn expire(store: &Store, args: &[String]) -> RespFrame {
    if args.len() != 3 {
        return RespFrame::Error("ERR wrong number of arguments for 'EXPIRE' command".to_string());
    }

    match args[2].parse::<u64>() {
        Ok(seconds) => RespFrame::Integer(store.expire(args[1].as_bytes(), seconds)),
        Err(_) => RespFrame::Error("ERR value is not an integer or out of range".to_string()),
    }
}

fn ttl(store: &Store, args: &[String]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'TTL' command".to_string());
    }

    RespFrame::Integer(store.ttl(args[1].as_bytes()))
}

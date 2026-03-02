use engine::value::CompactArg;
use protocol::types::RespFrame;

use super::super::pubsub::PubSubHub;

pub(super) fn emit_command_notifications(
    hub: &PubSubHub,
    command: &[u8],
    args: &[CompactArg],
    response: &RespFrame,
) {
    let Some((event, class, keys)) = keyspace_event_for_command(command, args, response) else {
        return;
    };

    for key in keys {
        hub.emit_keyspace_event(event, key, class);
    }
}

fn keyspace_event_for_command<'a>(
    command: &[u8],
    args: &'a [CompactArg],
    response: &RespFrame,
) -> Option<(&'static [u8], u8, Vec<&'a [u8]>)> {
    let ok = !matches!(response, RespFrame::Error(_));
    if !ok || args.len() < 2 {
        return None;
    }

    if command == b"SET"
        || command == b"SETEX"
        || command == b"PSETEX"
        || command == b"GETSET"
        || command == b"SETRANGE"
        || command == b"APPEND"
        || command == b"MSET"
        || command == b"MSETNX"
        || command == b"INCR"
        || command == b"INCRBY"
        || command == b"DECR"
        || command == b"DECRBY"
    {
        if command == b"MSET" || command == b"MSETNX" {
            let keys = args
                .iter()
                .skip(1)
                .step_by(2)
                .map(CompactArg::as_slice)
                .collect::<Vec<_>>();
            return Some((b"set", b'$', keys));
        }
        return Some((b"set", b'$', vec![args[1].as_slice()]));
    }

    if command == b"DEL" || command == b"UNLINK" {
        if matches!(response, RespFrame::Integer(value) if *value <= 0) {
            return None;
        }
        let keys = args
            .iter()
            .skip(1)
            .map(CompactArg::as_slice)
            .collect::<Vec<_>>();
        return Some((b"del", b'g', keys));
    }

    if command == b"EXPIRE"
        || command == b"PEXPIRE"
        || command == b"EXPIREAT"
        || command == b"PEXPIREAT"
    {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"expire", b'g', vec![args[1].as_slice()]));
        }
        return None;
    }

    if command == b"PERSIST" {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"persist", b'g', vec![args[1].as_slice()]));
        }
        return None;
    }

    if command == b"RENAME" || command == b"RENAMENX" {
        let success = matches!(response, RespFrame::Simple(value) if value == "OK")
            || matches!(response, RespFrame::Integer(1));
        if success && args.len() >= 3 {
            return Some((b"rename", b'g', vec![args[2].as_slice()]));
        }
        return None;
    }

    if command == b"HSET"
        || command == b"HSETNX"
        || command == b"HDEL"
        || command == b"HINCRBY"
        || command == b"HINCRBYFLOAT"
    {
        return Some((b"hset", b'h', vec![args[1].as_slice()]));
    }

    if command == b"LPUSH"
        || command == b"RPUSH"
        || command == b"LPOP"
        || command == b"RPOP"
        || command == b"LSET"
        || command == b"LTRIM"
        || command == b"LINSERT"
        || command == b"LMOVE"
        || command == b"RPOPLPUSH"
    {
        return Some((b"lset", b'l', vec![args[1].as_slice()]));
    }

    if command == b"SADD" || command == b"SREM" || command == b"SPOP" || command == b"SMOVE" {
        return Some((b"sadd", b's', vec![args[1].as_slice()]));
    }

    if command == b"ZADD"
        || command == b"ZREM"
        || command == b"ZINCRBY"
        || command == b"ZPOPMIN"
        || command == b"ZPOPMAX"
    {
        return Some((b"zadd", b'z', vec![args[1].as_slice()]));
    }

    None
}

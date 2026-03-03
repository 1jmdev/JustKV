use engine::value::CompactArg;
use protocol::types::RespFrame;

use super::super::pubsub::PubSubHub;

pub(super) fn emit_command_notifications(
    hub: &PubSubHub,
    command: &[u8],
    args: &[CompactArg],
    response: &RespFrame,
) {
    let _trace = profiler::scope("server::connection::notifications::emit_command_notifications");
    let Some((event, class)) = keyspace_event_for_command(command, args, response) else {
        return;
    };

    if command == b"MSET" || command == b"MSETNX" {
        for key in args.iter().skip(1).step_by(2) {
            hub.emit_keyspace_event(event, key.as_slice(), class);
        }
        return;
    }

    if command == b"DEL" || command == b"UNLINK" {
        for key in args.iter().skip(1) {
            hub.emit_keyspace_event(event, key.as_slice(), class);
        }
        return;
    }

    if command == b"RENAME" || command == b"RENAMENX" {
        hub.emit_keyspace_event(event, args[2].as_slice(), class);
        return;
    }

    hub.emit_keyspace_event(event, args[1].as_slice(), class);
}

fn keyspace_event_for_command(
    command: &[u8],
    args: &[CompactArg],
    response: &RespFrame,
) -> Option<(&'static [u8], u8)> {
    let _trace = profiler::scope("server::connection::notifications::keyspace_event_for_command");
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
        return Some((b"set", b'$'));
    }

    if command == b"DEL" || command == b"UNLINK" {
        if matches!(response, RespFrame::Integer(value) if *value <= 0) {
            return None;
        }
        return Some((b"del", b'g'));
    }

    if command == b"EXPIRE"
        || command == b"PEXPIRE"
        || command == b"EXPIREAT"
        || command == b"PEXPIREAT"
    {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"expire", b'g'));
        }
        return None;
    }

    if command == b"PERSIST" {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"persist", b'g'));
        }
        return None;
    }

    if command == b"RENAME" || command == b"RENAMENX" {
        let success = matches!(response, RespFrame::Simple(value) if value == "OK")
            || matches!(response, RespFrame::Integer(1));
        if success && args.len() >= 3 {
            return Some((b"rename", b'g'));
        }
        return None;
    }

    if command == b"HSET"
        || command == b"HSETNX"
        || command == b"HDEL"
        || command == b"HINCRBY"
        || command == b"HINCRBYFLOAT"
    {
        return Some((b"hset", b'h'));
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
        return Some((b"lset", b'l'));
    }

    if command == b"SADD" || command == b"SREM" || command == b"SPOP" || command == b"SMOVE" {
        return Some((b"sadd", b's'));
    }

    if command == b"ZADD"
        || command == b"ZREM"
        || command == b"ZINCRBY"
        || command == b"ZPOPMIN"
        || command == b"ZPOPMAX"
    {
        return Some((b"zadd", b'z'));
    }

    None
}

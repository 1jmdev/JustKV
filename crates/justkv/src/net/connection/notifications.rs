use crate::protocol::types::RespFrame;

use super::super::pubsub::PubSubHub;

pub(super) fn emit_command_notifications(
    hub: &PubSubHub,
    command: &[u8],
    args: &[Vec<u8>],
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
    args: &'a [Vec<u8>],
    response: &RespFrame,
) -> Option<(&'static [u8], u8, Vec<&'a [u8]>)> {
    let ok = !matches!(response, RespFrame::Error(_));
    if !ok || args.len() < 2 {
        return None;
    }

    if command.eq_ignore_ascii_case(b"SET")
        || command.eq_ignore_ascii_case(b"SETEX")
        || command.eq_ignore_ascii_case(b"PSETEX")
        || command.eq_ignore_ascii_case(b"GETSET")
        || command.eq_ignore_ascii_case(b"SETRANGE")
        || command.eq_ignore_ascii_case(b"APPEND")
        || command.eq_ignore_ascii_case(b"MSET")
        || command.eq_ignore_ascii_case(b"MSETNX")
        || command.eq_ignore_ascii_case(b"INCR")
        || command.eq_ignore_ascii_case(b"INCRBY")
        || command.eq_ignore_ascii_case(b"DECR")
        || command.eq_ignore_ascii_case(b"DECRBY")
    {
        if command.eq_ignore_ascii_case(b"MSET") || command.eq_ignore_ascii_case(b"MSETNX") {
            let keys = args
                .iter()
                .skip(1)
                .step_by(2)
                .map(Vec::as_slice)
                .collect::<Vec<_>>();
            return Some((b"set", b'$', keys));
        }
        return Some((b"set", b'$', vec![args[1].as_slice()]));
    }

    if command.eq_ignore_ascii_case(b"DEL") || command.eq_ignore_ascii_case(b"UNLINK") {
        if matches!(response, RespFrame::Integer(value) if *value <= 0) {
            return None;
        }
        let keys = args.iter().skip(1).map(Vec::as_slice).collect::<Vec<_>>();
        return Some((b"del", b'g', keys));
    }

    if command.eq_ignore_ascii_case(b"EXPIRE")
        || command.eq_ignore_ascii_case(b"PEXPIRE")
        || command.eq_ignore_ascii_case(b"EXPIREAT")
        || command.eq_ignore_ascii_case(b"PEXPIREAT")
    {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"expire", b'g', vec![args[1].as_slice()]));
        }
        return None;
    }

    if command.eq_ignore_ascii_case(b"PERSIST") {
        if matches!(response, RespFrame::Integer(1)) {
            return Some((b"persist", b'g', vec![args[1].as_slice()]));
        }
        return None;
    }

    if command.eq_ignore_ascii_case(b"RENAME") || command.eq_ignore_ascii_case(b"RENAMENX") {
        let success = matches!(response, RespFrame::Simple(value) if value == "OK")
            || matches!(response, RespFrame::Integer(1));
        if success && args.len() >= 3 {
            return Some((b"rename", b'g', vec![args[2].as_slice()]));
        }
        return None;
    }

    if command.eq_ignore_ascii_case(b"HSET")
        || command.eq_ignore_ascii_case(b"HSETNX")
        || command.eq_ignore_ascii_case(b"HDEL")
        || command.eq_ignore_ascii_case(b"HINCRBY")
        || command.eq_ignore_ascii_case(b"HINCRBYFLOAT")
    {
        return Some((b"hset", b'h', vec![args[1].as_slice()]));
    }

    if command.eq_ignore_ascii_case(b"LPUSH")
        || command.eq_ignore_ascii_case(b"RPUSH")
        || command.eq_ignore_ascii_case(b"LPOP")
        || command.eq_ignore_ascii_case(b"RPOP")
        || command.eq_ignore_ascii_case(b"LSET")
        || command.eq_ignore_ascii_case(b"LTRIM")
        || command.eq_ignore_ascii_case(b"LINSERT")
        || command.eq_ignore_ascii_case(b"LMOVE")
        || command.eq_ignore_ascii_case(b"RPOPLPUSH")
    {
        return Some((b"lset", b'l', vec![args[1].as_slice()]));
    }

    if command.eq_ignore_ascii_case(b"SADD")
        || command.eq_ignore_ascii_case(b"SREM")
        || command.eq_ignore_ascii_case(b"SPOP")
        || command.eq_ignore_ascii_case(b"SMOVE")
    {
        return Some((b"sadd", b's', vec![args[1].as_slice()]));
    }

    if command.eq_ignore_ascii_case(b"ZADD")
        || command.eq_ignore_ascii_case(b"ZREM")
        || command.eq_ignore_ascii_case(b"ZINCRBY")
        || command.eq_ignore_ascii_case(b"ZPOPMIN")
        || command.eq_ignore_ascii_case(b"ZPOPMAX")
    {
        return Some((b"zadd", b'z', vec![args[1].as_slice()]));
    }

    None
}

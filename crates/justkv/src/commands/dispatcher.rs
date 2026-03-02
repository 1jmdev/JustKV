use crate::commands::util::{CommandId, parse_command_id};
use crate::commands::{connection, hash, keyspace, list, set, string, ttl, zset};
use crate::engine::store::Store;
use crate::engine::value::CompactArg;
use crate::protocol::types::{BulkData, RespFrame};

pub fn dispatch(store: &Store, frame: RespFrame) -> RespFrame {
    let mut args = Vec::new();
    if let Err(err) = parse_command_into(frame, &mut args) {
        return RespFrame::error_static(err);
    }

    dispatch_args(store, &args)
}

#[inline]
pub fn dispatch_args(store: &Store, args: &[CompactArg]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    let cmd = parse_command_id(args[0].as_slice());

    match cmd {
        CommandId::Get => return string::get(store, args),
        CommandId::Set => return string::set(store, args),
        CommandId::Incr => return string::incr(store, args),
        CommandId::Del => return keyspace::del(store, args),
        CommandId::Expire => return ttl::expire(store, args),
        CommandId::Ping => return connection::ping(args),
        _ => {}
    }

    dispatch_cold(store, cmd, args)
}

#[cold]
fn dispatch_cold(store: &Store, cmd: CommandId, args: &[CompactArg]) -> RespFrame {
    match cmd {
        // ── Connection ────────────────────────────────────────────────────
        CommandId::Auth => connection::auth(args),
        CommandId::Hello => connection::hello(args),
        CommandId::Client => connection::client(args),
        CommandId::Command => RespFrame::Array(Some(vec![])),
        CommandId::Select => connection::select_db(args),
        CommandId::Quit => connection::quit(args),
        CommandId::Echo => connection::echo(args),

        // ── Strings ───────────────────────────────────────────────────────
        CommandId::Setnx => string::setnx(store, args),
        CommandId::Getset => string::getset(store, args),
        CommandId::Getdel => string::getdel(store, args),
        CommandId::Setex => string::setex(store, args),
        CommandId::Psetex => string::psetex(store, args),
        CommandId::Getex => string::getex(store, args),
        CommandId::Append => string::append(store, args),
        CommandId::Strlen => string::strlen(store, args),
        CommandId::Setrange => string::setrange(store, args),
        CommandId::Getrange => string::getrange(store, args),
        CommandId::Mget => string::mget(store, args),
        CommandId::Mset => string::mset(store, args),
        CommandId::Msetnx => string::msetnx(store, args),
        CommandId::Incrby => string::incrby(store, args),
        CommandId::Decr => string::decr(store, args),
        CommandId::Decrby => string::decrby(store, args),

        // ── Hashes ────────────────────────────────────────────────────────
        CommandId::Hset => hash::hset(store, args),
        CommandId::Hmset => hash::hmset(store, args),
        CommandId::Hsetnx => hash::hsetnx(store, args),
        CommandId::Hget => hash::hget(store, args),
        CommandId::Hmget => hash::hmget(store, args),
        CommandId::Hgetall => hash::hgetall(store, args),
        CommandId::Hdel => hash::hdel(store, args),
        CommandId::Hexists => hash::hexists(store, args),
        CommandId::Hkeys => hash::hkeys(store, args),
        CommandId::Hvals => hash::hvals(store, args),
        CommandId::Hlen => hash::hlen(store, args),
        CommandId::Hstrlen => hash::hstrlen(store, args),
        CommandId::Hincrby => hash::hincrby(store, args),
        CommandId::Hincrbyfloat => hash::hincrbyfloat(store, args),
        CommandId::Hrandfield => hash::hrandfield(store, args),
        CommandId::Hscan => hash::hscan(store, args),

        // ── Lists ─────────────────────────────────────────────────────────
        CommandId::Lpush => list::lpush(store, args),
        CommandId::Rpush => list::rpush(store, args),
        CommandId::Lpop => list::lpop(store, args),
        CommandId::Rpop => list::rpop(store, args),
        CommandId::Llen => list::llen(store, args),
        CommandId::Lindex => list::lindex(store, args),
        CommandId::Lrange => list::lrange(store, args),
        CommandId::Lset => list::lset(store, args),
        CommandId::Ltrim => list::ltrim(store, args),
        CommandId::Linsert => list::linsert(store, args),
        CommandId::Lpos => list::lpos(store, args),
        CommandId::Lmove => list::lmove(store, args),
        CommandId::Brpoplpush => list::brpoplpush(store, args),
        CommandId::Lmpop => list::lmpop(store, args),
        CommandId::Blpop => list::blpop(store, args),
        CommandId::Brpop => list::brpop(store, args),
        CommandId::Blmpop => list::blmpop(store, args),

        // ── Sets ──────────────────────────────────────────────────────────
        CommandId::Sadd => set::sadd(store, args),
        CommandId::Srem => set::srem(store, args),
        CommandId::Smembers => set::smembers(store, args),
        CommandId::Sismember => set::sismember(store, args),
        CommandId::Scard => set::scard(store, args),
        CommandId::Smove => set::smove(store, args),
        CommandId::Spop => set::spop(store, args),
        CommandId::Srandmember => set::srandmember(store, args),
        CommandId::Sinter => set::sinter(store, args),
        CommandId::Sinterstore => set::sinterstore(store, args),
        CommandId::Sunion => set::sunion(store, args),
        CommandId::Sunionstore => set::sunionstore(store, args),
        CommandId::Sdiff => set::sdiff(store, args),
        CommandId::Sdiffstore => set::sdiffstore(store, args),
        CommandId::Sintercard => set::sintercard(store, args),
        CommandId::Sscan => set::sscan(store, args),

        // ── Sorted sets ───────────────────────────────────────────────────
        CommandId::Zadd => zset::zadd(store, args),
        CommandId::Zrem => zset::zrem(store, args),
        CommandId::Zcard => zset::zcard(store, args),
        CommandId::Zcount => zset::zcount(store, args),
        CommandId::Zscore => zset::zscore(store, args),
        CommandId::Zrank => zset::zrank(store, args, false),
        CommandId::Zrevrank => zset::zrank(store, args, true),
        CommandId::Zincrby => zset::zincrby(store, args),
        CommandId::Zmscore => zset::zmscore(store, args),
        CommandId::Zrange => zset::zrange(store, args, false),
        CommandId::Zrevrange => zset::zrange(store, args, true),
        CommandId::Zrangebyscore => zset::zrange_by_score(store, args, false),
        CommandId::Zrevrangebyscore => zset::zrange_by_score(store, args, true),
        CommandId::Zpopmin => zset::zpop(store, args, false),
        CommandId::Zpopmax => zset::zpop(store, args, true),
        CommandId::Bzpopmin => zset::bzpop(store, args, false),
        CommandId::Bzpopmax => zset::bzpop(store, args, true),
        CommandId::Zmpop => zset::zmpop(store, args),
        CommandId::Bzmpop => zset::bzmpop(store, args),
        CommandId::Zrandmember => zset::zrandmember(store, args),
        CommandId::Zinter => zset::zop(store, args, "ZINTER"),
        CommandId::Zunion => zset::zop(store, args, "ZUNION"),
        CommandId::Zdiff => zset::zop(store, args, "ZDIFF"),
        CommandId::Zscan => zset::zscan(store, args),

        // ── Keyspace ──────────────────────────────────────────────────────
        CommandId::Exists => keyspace::exists(store, args),
        CommandId::Touch => keyspace::touch(store, args),
        CommandId::Unlink => keyspace::unlink(store, args),
        CommandId::Type => keyspace::key_type(store, args),
        CommandId::Rename => keyspace::rename(store, args),
        CommandId::Renamenx => keyspace::renamenx(store, args),
        CommandId::Dbsize => keyspace::dbsize(store, args),
        CommandId::Keys => keyspace::keys(store, args),
        CommandId::Scan => keyspace::scan(store, args),
        CommandId::Move => keyspace::move_key(store, args),
        CommandId::Dump => keyspace::dump(store, args),
        CommandId::Restore => keyspace::restore(store, args),
        CommandId::Sort => keyspace::sort(store, args),
        CommandId::Copy => keyspace::copy(store, args),
        CommandId::Flushdb => keyspace::flushdb(store, args),
        CommandId::Flushall => keyspace::flushall(store, args),

        // ── TTL ───────────────────────────────────────────────────────────
        CommandId::Pexpire => ttl::pexpire(store, args),
        CommandId::Expireat => ttl::expireat(store, args),
        CommandId::Pexpireat => ttl::pexpireat(store, args),
        CommandId::Persist => ttl::persist(store, args),
        CommandId::Ttl => ttl::ttl(store, args),
        CommandId::Pttl => ttl::pttl(store, args),

        CommandId::Unknown => RespFrame::error_static("ERR unknown command"),

        // These were handled by the hot path in dispatch_args before
        // dispatch_cold was ever called.
        CommandId::Get
        | CommandId::Set
        | CommandId::Incr
        | CommandId::Del
        | CommandId::Expire
        | CommandId::Ping => unreachable!(),
    }
}

pub fn parse_command(frame: RespFrame) -> Result<Vec<CompactArg>, &'static str> {
    let mut args = Vec::new();
    parse_command_into(frame, &mut args)?;
    Ok(args)
}

pub fn parse_command_into(
    frame: RespFrame,
    args: &mut Vec<CompactArg>,
) -> Result<(), &'static str> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error");
    };

    args.clear();
    if args.capacity() < items.len() {
        args.reserve(items.len() - args.capacity());
    }

    for item in items {
        match item {
            RespFrame::Bulk(Some(BulkData::Arg(bytes))) => args.push(bytes),
            RespFrame::Bulk(Some(BulkData::Value(bytes))) => {
                args.push(CompactArg::from_vec(bytes.into_vec()))
            }
            RespFrame::Simple(value) => args.push(CompactArg::from_vec(value.into_bytes())),
            RespFrame::SimpleStatic(value) => {
                args.push(CompactArg::from_slice(value.as_bytes()));
            }
            _ => return Err("ERR invalid argument type"),
        }
    }

    if let Some(first) = args.first_mut() {
        first.make_ascii_uppercase();
    }

    Ok(())
}

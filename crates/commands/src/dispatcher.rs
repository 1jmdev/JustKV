use crate::util::{cmd, pack_runtime};
use crate::{connection, geo, hash, keyspace, list, set, stream, string, ttl, zset};
use engine::store::Store;
use engine::value::CompactArg;
use protocol::types::{BulkData, RespFrame};

pub fn dispatch(store: &Store, frame: RespFrame) -> RespFrame {
    let _trace = profiler::scope("commands::dispatcher::dispatch");
    let mut args = Vec::new();
    if let Err(err) = parse_command_into(frame, &mut args) {
        return RespFrame::error_static(err);
    }

    dispatch_args(store, &args)
}

#[inline]
pub fn dispatch_args(store: &Store, args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("commands::dispatcher::dispatch_args");
    if args.is_empty() {
        return RespFrame::error_static("ERR empty command");
    }

    let raw = args[0].as_slice();

    // Hot path: commands ≤8 bytes packed to a u64 integer.
    // LLVM compiles this match into a jump table or optimised binary search —
    // one integer compare per arm, no multi-level branching.
    if raw.len() <= 8 {
        let key = pack_runtime(raw);
        match key {
            // ── Highest-frequency commands first ──────────────────────────────
            cmd::GET => return string::get(store, args),
            cmd::SET => return string::set(store, args),
            cmd::INCR => return string::incr(store, args),
            cmd::DEL => return keyspace::del(store, args),
            cmd::EXPIRE => return ttl::expire(store, args),
            cmd::PING => return connection::ping(args),

            // ── Connection ────────────────────────────────────────────────────
            cmd::AUTH => return connection::auth(args),
            cmd::HELLO => return connection::hello(args),
            cmd::CLIENT => return connection::client(args),
            cmd::COMMAND => return RespFrame::Array(Some(vec![])),
            cmd::SELECT => return connection::select_db(args),
            cmd::QUIT => return connection::quit(args),
            cmd::ECHO => return connection::echo(args),

            // ── String ────────────────────────────────────────────────────────
            cmd::SETNX => return string::setnx(store, args),
            cmd::GETSET => return string::getset(store, args),
            cmd::GETDEL => return string::getdel(store, args),
            cmd::SETEX => return string::setex(store, args),
            cmd::PSETEX => return string::psetex(store, args),
            cmd::GETEX => return string::getex(store, args),
            cmd::APPEND => return string::append(store, args),
            cmd::STRLEN => return string::strlen(store, args),
            cmd::SETRANGE => return string::setrange(store, args),
            cmd::GETRANGE => return string::getrange(store, args),
            cmd::MGET => return string::mget(store, args),
            cmd::MSET => return string::mset(store, args),
            cmd::MSETNX => return string::msetnx(store, args),
            cmd::INCRBY => return string::incrby(store, args),
            cmd::DECR => return string::decr(store, args),
            cmd::DECRBY => return string::decrby(store, args),
            cmd::SETBIT => return string::setbit(store, args),
            cmd::GETBIT => return string::getbit(store, args),
            cmd::BITCOUNT => return string::bitcount(store, args),
            cmd::BITPOS => return string::bitpos(store, args),
            cmd::BITOP => return string::bitop(store, args),
            cmd::BITFIELD => return string::bitfield(store, args),
            cmd::PFADD => return string::pfadd(store, args),
            cmd::PFCOUNT => return string::pfcount(store, args),
            cmd::PFMERGE => return string::pfmerge(store, args),

            // ── Hash ──────────────────────────────────────────────────────────
            cmd::HSET => return hash::hset(store, args),
            cmd::HMSET => return hash::hmset(store, args),
            cmd::HSETNX => return hash::hsetnx(store, args),
            cmd::HGET => return hash::hget(store, args),
            cmd::HMGET => return hash::hmget(store, args),
            cmd::HGETALL => return hash::hgetall(store, args),
            cmd::HDEL => return hash::hdel(store, args),
            cmd::HEXISTS => return hash::hexists(store, args),
            cmd::HKEYS => return hash::hkeys(store, args),
            cmd::HVALS => return hash::hvals(store, args),
            cmd::HLEN => return hash::hlen(store, args),
            cmd::HSTRLEN => return hash::hstrlen(store, args),
            cmd::HINCRBY => return hash::hincrby(store, args),
            cmd::HSCAN => return hash::hscan(store, args),

            // ── List ──────────────────────────────────────────────────────────
            cmd::LPUSH => return list::lpush(store, args),
            cmd::RPUSH => return list::rpush(store, args),
            cmd::LPOP => return list::lpop(store, args),
            cmd::RPOP => return list::rpop(store, args),
            cmd::LLEN => return list::llen(store, args),
            cmd::LINDEX => return list::lindex(store, args),
            cmd::LRANGE => return list::lrange(store, args),
            cmd::LSET => return list::lset(store, args),
            cmd::LTRIM => return list::ltrim(store, args),
            cmd::LINSERT => return list::linsert(store, args),
            cmd::LPOS => return list::lpos(store, args),
            cmd::LMOVE => return list::lmove(store, args),
            cmd::LMPOP => return list::lmpop(store, args),
            cmd::BLPOP => return list::blpop(store, args),
            cmd::BRPOP => return list::brpop(store, args),
            cmd::BLMPOP => return list::blmpop(store, args),

            // ── Set ───────────────────────────────────────────────────────────
            cmd::SADD => return set::sadd(store, args),
            cmd::SREM => return set::srem(store, args),
            cmd::SMEMBERS => return set::smembers(store, args),
            cmd::SCARD => return set::scard(store, args),
            cmd::SMOVE => return set::smove(store, args),
            cmd::SPOP => return set::spop(store, args),
            cmd::SINTER => return set::sinter(store, args),
            cmd::SUNION => return set::sunion(store, args),
            cmd::SDIFF => return set::sdiff(store, args),
            cmd::SSCAN => return set::sscan(store, args),

            // ── Sorted set ────────────────────────────────────────────────────
            cmd::ZADD => return zset::zadd(store, args),
            cmd::ZREM => return zset::zrem(store, args),
            cmd::ZCARD => return zset::zcard(store, args),
            cmd::ZCOUNT => return zset::zcount(store, args),
            cmd::ZSCORE => return zset::zscore(store, args),
            cmd::ZRANK => return zset::zrank(store, args, false),
            cmd::ZREVRANK => return zset::zrank(store, args, true),
            cmd::ZINCRBY => return zset::zincrby(store, args),
            cmd::ZMSCORE => return zset::zmscore(store, args),
            cmd::ZRANGE => return zset::zrange(store, args, false),
            cmd::ZPOPMIN => return zset::zpop(store, args, false),
            cmd::ZPOPMAX => return zset::zpop(store, args, true),
            cmd::BZPOPMIN => return zset::bzpop(store, args, false),
            cmd::BZPOPMAX => return zset::bzpop(store, args, true),
            cmd::ZMPOP => return zset::zmpop(store, args),
            cmd::BZMPOP => return zset::bzmpop(store, args),
            cmd::ZINTER => return zset::zop(store, args, "ZINTER"),
            cmd::ZUNION => return zset::zop(store, args, "ZUNION"),
            cmd::ZDIFF => return zset::zop(store, args, "ZDIFF"),
            cmd::ZSCAN => return zset::zscan(store, args),

            // ── GEO ───────────────────────────────────────────────────────────
            cmd::GEOADD => return geo::geoadd(store, args),
            cmd::GEOPOS => return geo::geopos(store, args),
            cmd::GEODIST => return geo::geodist(store, args),
            cmd::GEOHASH => return geo::geohash(store, args),

            // ── Stream ────────────────────────────────────────────────────────
            cmd::XADD => return stream::xadd(store, args),
            cmd::XLEN => return stream::xlen(store, args),
            cmd::XDEL => return stream::xdel(store, args),
            cmd::XRANGE => return stream::xrange(store, args),
            cmd::XTRIM => return stream::xtrim(store, args),
            cmd::XREAD => return stream::xread(store, args),
            cmd::XGROUP => return stream::xgroup(store, args),
            cmd::XACK => return stream::xack(store, args),
            cmd::XCLAIM => return stream::xclaim(store, args),
            cmd::XPENDING => return stream::xpending(store, args),

            // ── Keyspace ──────────────────────────────────────────────────────
            cmd::EXISTS => return keyspace::exists(store, args),
            cmd::TOUCH => return keyspace::touch(store, args),
            cmd::UNLINK => return keyspace::unlink(store, args),
            cmd::TYPE => return keyspace::key_type(store, args),
            cmd::RENAME => return keyspace::rename(store, args),
            cmd::RENAMENX => return keyspace::renamenx(store, args),
            cmd::DBSIZE => return keyspace::dbsize(store, args),
            cmd::KEYS => return keyspace::keys(store, args),
            cmd::SCAN => return keyspace::scan(store, args),
            cmd::MOVE => return keyspace::move_key(store, args),
            cmd::DUMP => return keyspace::dump(store, args),
            cmd::RESTORE => return keyspace::restore(store, args),
            cmd::SORT => return keyspace::sort(store, args),
            cmd::COPY => return keyspace::copy(store, args),
            cmd::FLUSHDB => return keyspace::flushdb(store, args),
            cmd::FLUSHALL => return keyspace::flushall(store, args),

            // ── TTL ───────────────────────────────────────────────────────────
            cmd::PEXPIRE => return ttl::pexpire(store, args),
            cmd::EXPIREAT => return ttl::expireat(store, args),
            cmd::PERSIST => return ttl::persist(store, args),
            cmd::TTL => return ttl::ttl(store, args),
            cmd::PTTL => return ttl::pttl(store, args),

            _ => {}
        }
    }

    dispatch_long(store, raw, args)
}

/// Slow path for commands longer than 8 bytes.
/// These are rare so a simple eq_ignore_ascii_case chain is fine.
#[cold]
fn dispatch_long(store: &Store, raw: &[u8], args: &[CompactArg]) -> RespFrame {
    let _trace = profiler::scope("commands::dispatcher::dispatch_long");
    if raw.eq_ignore_ascii_case(b"PEXPIREAT") {
        return ttl::pexpireat(store, args);
    }
    if raw.eq_ignore_ascii_case(b"BITFIELD_RO") {
        return string::bitfield_ro(store, args);
    }
    if raw.eq_ignore_ascii_case(b"HINCRBYFLOAT") {
        return hash::hincrbyfloat(store, args);
    }
    if raw.eq_ignore_ascii_case(b"HRANDFIELD") {
        return hash::hrandfield(store, args);
    }
    if raw.eq_ignore_ascii_case(b"BRPOPLPUSH") {
        return list::brpoplpush(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SISMEMBER") {
        return set::sismember(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SDIFFSTORE") {
        return set::sdiffstore(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SINTERCARD") {
        return set::sintercard(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SINTERSTORE") {
        return set::sinterstore(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SUNIONSTORE") {
        return set::sunionstore(store, args);
    }
    if raw.eq_ignore_ascii_case(b"SRANDMEMBER") {
        return set::srandmember(store, args);
    }
    if raw.eq_ignore_ascii_case(b"ZREVRANGE") {
        return zset::zrange(store, args, true);
    }
    if raw.eq_ignore_ascii_case(b"ZRANGEBYSCORE") {
        return zset::zrange_by_score(store, args, false);
    }
    if raw.eq_ignore_ascii_case(b"ZREVRANGEBYSCORE") {
        return zset::zrange_by_score(store, args, true);
    }
    if raw.eq_ignore_ascii_case(b"ZRANDMEMBER") {
        return zset::zrandmember(store, args);
    }
    if raw.eq_ignore_ascii_case(b"ZREMRANGEBYRANK") {
        return zset::zremrangebyrank(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEORADIUS") {
        return geo::georadius(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEORADIUS_RO") {
        return geo::georadius_ro(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEORADIUSBYMEMBER") {
        return geo::georadiusbymember(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEORADIUSBYMEMBER_RO") {
        return geo::georadiusbymember_ro(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEOSEARCH") {
        return geo::geosearch(store, args);
    }
    if raw.eq_ignore_ascii_case(b"GEOSEARCHSTORE") {
        return geo::geosearchstore(store, args);
    }
    if raw.eq_ignore_ascii_case(b"XREVRANGE") {
        return stream::xrevrange(store, args);
    }
    if raw.eq_ignore_ascii_case(b"XREADGROUP") {
        return stream::xreadgroup(store, args);
    }
    if raw.eq_ignore_ascii_case(b"XAUTOCLAIM") {
        return stream::xautoclaim(store, args);
    }

    RespFrame::error_static("ERR unknown command")
}

pub fn parse_command(frame: RespFrame) -> Result<Vec<CompactArg>, &'static str> {
    let _trace = profiler::scope("commands::dispatcher::parse_command");
    let mut args = Vec::new();
    parse_command_into(frame, &mut args)?;
    Ok(args)
}

pub fn parse_command_into(
    frame: RespFrame,
    args: &mut Vec<CompactArg>,
) -> Result<(), &'static str> {
    let _trace = profiler::scope("commands::dispatcher::parse_command_into");
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

use crate::command::{CommandId, identify};
use crate::{connection, geo, hash, keyspace, list, scripting, set, stream, string, ttl, zset};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

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

    dispatch_with_id(store, identify(args[0].as_slice()), args)
}

#[inline]
pub fn dispatch_with_id(store: &Store, command: CommandId, args: &[CompactArg]) -> RespFrame {
    match command {
        CommandId::Get => string::get(store, args),
        CommandId::Set => string::set(store, args),
        CommandId::Incr => string::incr(store, args),
        CommandId::IncrByFloat => string::incrbyfloat(store, args),
        CommandId::Del => keyspace::del(store, args),
        CommandId::Expire => ttl::expire(store, args),
        CommandId::Ping => connection::ping(args),
        CommandId::Eval => scripting::eval(store, args),
        CommandId::EvalRo => scripting::eval_ro(store, args),
        CommandId::EvalSha => scripting::evalsha(store, args),
        CommandId::EvalShaRo => scripting::evalsha_ro(store, args),
        CommandId::Script => scripting::script(store, args),
        CommandId::Auth => connection::auth(args),
        CommandId::Hello => connection::hello(args),
        CommandId::Client => connection::client(args),
        CommandId::Command => {
            if args.len() == 2 && args[1].eq_ignore_ascii_case(b"COUNT") {
                RespFrame::Integer(0)
            } else {
                RespFrame::Array(Some(vec![]))
            }
        }
        CommandId::Select => connection::select_db(args),
        CommandId::Quit => connection::quit(args),
        CommandId::Echo => connection::echo(args),
        CommandId::SetNx => string::setnx(store, args),
        CommandId::GetSet => string::getset(store, args),
        CommandId::GetDel => string::getdel(store, args),
        CommandId::SetEx => string::setex(store, args),
        CommandId::PSetEx => string::psetex(store, args),
        CommandId::GetEx => string::getex(store, args),
        CommandId::Append => string::append(store, args),
        CommandId::StrLen => string::strlen(store, args),
        CommandId::SetRange => string::setrange(store, args),
        CommandId::GetRange => string::getrange(store, args),
        CommandId::MGet => string::mget(store, args),
        CommandId::MSet => string::mset(store, args),
        CommandId::MSetNx => string::msetnx(store, args),
        CommandId::IncrBy => string::incrby(store, args),
        CommandId::Decr => string::decr(store, args),
        CommandId::DecrBy => string::decrby(store, args),
        CommandId::SetBit => string::setbit(store, args),
        CommandId::GetBit => string::getbit(store, args),
        CommandId::BitCount => string::bitcount(store, args),
        CommandId::BitPos => string::bitpos(store, args),
        CommandId::BitOp => string::bitop(store, args),
        CommandId::BitField => string::bitfield(store, args),
        CommandId::BitFieldRo => string::bitfield_ro(store, args),
        CommandId::PfAdd => string::pfadd(store, args),
        CommandId::PfCount => string::pfcount(store, args),
        CommandId::PfMerge => string::pfmerge(store, args),
        CommandId::HSet => hash::hset(store, args),
        CommandId::HMSet => hash::hmset(store, args),
        CommandId::HSetNx => hash::hsetnx(store, args),
        CommandId::HGet => hash::hget(store, args),
        CommandId::HMGet => hash::hmget(store, args),
        CommandId::HGetAll => hash::hgetall(store, args),
        CommandId::HDel => hash::hdel(store, args),
        CommandId::HExists => hash::hexists(store, args),
        CommandId::HKeys => hash::hkeys(store, args),
        CommandId::HVals => hash::hvals(store, args),
        CommandId::HLen => hash::hlen(store, args),
        CommandId::HStrLen => hash::hstrlen(store, args),
        CommandId::HIncrBy => hash::hincrby(store, args),
        CommandId::HIncrByFloat => hash::hincrbyfloat(store, args),
        CommandId::HScan => hash::hscan(store, args),
        CommandId::HRandField => hash::hrandfield(store, args),
        CommandId::LPush => list::lpush(store, args),
        CommandId::LPushX => list::lpushx(store, args),
        CommandId::RPush => list::rpush(store, args),
        CommandId::RPushX => list::rpushx(store, args),
        CommandId::LPop => list::lpop(store, args),
        CommandId::RPop => list::rpop(store, args),
        CommandId::LRem => list::lrem(store, args),
        CommandId::LLen => list::llen(store, args),
        CommandId::LIndex => list::lindex(store, args),
        CommandId::LRange => list::lrange(store, args),
        CommandId::LSet => list::lset(store, args),
        CommandId::LTrim => list::ltrim(store, args),
        CommandId::LInsert => list::linsert(store, args),
        CommandId::LPos => list::lpos(store, args),
        CommandId::LMove => list::lmove(store, args),
        CommandId::LMPop => list::lmpop(store, args),
        CommandId::BLPop => list::blpop(store, args),
        CommandId::BRPop => list::brpop(store, args),
        CommandId::BLMPop => list::blmpop(store, args),
        CommandId::BRPopLPush => list::brpoplpush(store, args),
        CommandId::RPopLPush => list::rpoplpush(store, args),
        CommandId::SAdd => set::sadd(store, args),
        CommandId::SRem => set::srem(store, args),
        CommandId::SMembers => set::smembers(store, args),
        CommandId::SCard => set::scard(store, args),
        CommandId::SMove => set::smove(store, args),
        CommandId::SPop => set::spop(store, args),
        CommandId::SInter => set::sinter(store, args),
        CommandId::SUnion => set::sunion(store, args),
        CommandId::SDiff => set::sdiff(store, args),
        CommandId::SScan => set::sscan(store, args),
        CommandId::SIsMember => set::sismember(store, args),
        CommandId::SMIsMember => set::smismember(store, args),
        CommandId::SDiffStore => set::sdiffstore(store, args),
        CommandId::SInterCard => set::sintercard(store, args),
        CommandId::SInterStore => set::sinterstore(store, args),
        CommandId::SUnionStore => set::sunionstore(store, args),
        CommandId::SRandMember => set::srandmember(store, args),
        CommandId::ZAdd => zset::zadd(store, args),
        CommandId::ZRem => zset::zrem(store, args),
        CommandId::ZCard => zset::zcard(store, args),
        CommandId::ZCount => zset::zcount(store, args),
        CommandId::ZScore => zset::zscore(store, args),
        CommandId::ZRank => zset::zrank(store, args, false),
        CommandId::ZRevRank => zset::zrank(store, args, true),
        CommandId::ZIncrBy => zset::zincrby(store, args),
        CommandId::ZMScore => zset::zmscore(store, args),
        CommandId::ZRange => zset::zrange(store, args, false),
        CommandId::ZRevRange => zset::zrange(store, args, true),
        CommandId::ZPopMin => zset::zpop(store, args, false),
        CommandId::ZPopMax => zset::zpop(store, args, true),
        CommandId::BZPopMin => zset::bzpop(store, args, false),
        CommandId::BZPopMax => zset::bzpop(store, args, true),
        CommandId::ZMPop => zset::zmpop(store, args),
        CommandId::BZMPop => zset::bzmpop(store, args),
        CommandId::ZInter => zset::zop(store, args, "ZINTER"),
        CommandId::ZInterStore => zset::zop_store(store, args, "ZINTERSTORE"),
        CommandId::ZUnion => zset::zop(store, args, "ZUNION"),
        CommandId::ZUnionStore => zset::zop_store(store, args, "ZUNIONSTORE"),
        CommandId::ZDiff => zset::zop(store, args, "ZDIFF"),
        CommandId::ZDiffStore => zset::zop_store(store, args, "ZDIFFSTORE"),
        CommandId::ZScan => zset::zscan(store, args),
        CommandId::ZRandMember => zset::zrandmember(store, args),
        CommandId::ZRangeStore => zset::zrangestore(store, args),
        CommandId::ZRangeByScore => zset::zrange_by_score(store, args, false),
        CommandId::ZRangeByLex => zset::zrange_by_lex(store, args, false),
        CommandId::ZRevRangeByScore => zset::zrange_by_score(store, args, true),
        CommandId::ZRevRangeByLex => zset::zrange_by_lex(store, args, true),
        CommandId::ZLexCount => zset::zlexcount(store, args),
        CommandId::ZRemRangeByLex => zset::zremrangebylex(store, args),
        CommandId::ZRemRangeByRank => zset::zremrangebyrank(store, args),
        CommandId::ZRemRangeByScore => zset::zremrangebyscore(store, args),
        CommandId::GeoAdd => geo::geoadd(store, args),
        CommandId::GeoPos => geo::geopos(store, args),
        CommandId::GeoDist => geo::geodist(store, args),
        CommandId::GeoHash => geo::geohash(store, args),
        CommandId::GeoRadius => geo::georadius(store, args),
        CommandId::GeoRadiusRo => geo::georadius_ro(store, args),
        CommandId::GeoSearch => geo::geosearch(store, args),
        CommandId::GeoSearchStore => geo::geosearchstore(store, args),
        CommandId::GeoRadiusByMember => geo::georadiusbymember(store, args),
        CommandId::GeoRadiusByMemberRo => geo::georadiusbymember_ro(store, args),
        CommandId::XAdd => stream::xadd(store, args),
        CommandId::XLen => stream::xlen(store, args),
        CommandId::XDel => stream::xdel(store, args),
        CommandId::XRange => stream::xrange(store, args),
        CommandId::XRevRange => stream::xrevrange(store, args),
        CommandId::XTrim => stream::xtrim(store, args),
        CommandId::XRead => stream::xread(store, args),
        CommandId::XGroup => stream::xgroup(store, args),
        CommandId::XAck => stream::xack(store, args),
        CommandId::XClaim => stream::xclaim(store, args),
        CommandId::XPending => stream::xpending(store, args),
        CommandId::XReadGroup => stream::xreadgroup(store, args),
        CommandId::XAutoClaim => stream::xautoclaim(store, args),
        CommandId::Exists => keyspace::exists(store, args),
        CommandId::Touch => keyspace::touch(store, args),
        CommandId::Unlink => keyspace::unlink(store, args),
        CommandId::Type => keyspace::key_type(store, args),
        CommandId::Rename => keyspace::rename(store, args),
        CommandId::RenameNx => keyspace::renamenx(store, args),
        CommandId::DbSize => keyspace::dbsize(store, args),
        CommandId::Keys => keyspace::keys(store, args),
        CommandId::Scan => keyspace::scan(store, args),
        CommandId::Move => keyspace::move_key(store, args),
        CommandId::Dump => keyspace::dump(store, args),
        CommandId::Restore => keyspace::restore(store, args),
        CommandId::Sort => keyspace::sort(store, args),
        CommandId::Copy => keyspace::copy(store, args),
        CommandId::FlushDb => keyspace::flushdb(store, args),
        CommandId::FlushAll => keyspace::flushall(store, args),
        CommandId::PExpire => ttl::pexpire(store, args),
        CommandId::ExpireAt => ttl::expireat(store, args),
        CommandId::PExpireAt => ttl::pexpireat(store, args),
        CommandId::Persist => ttl::persist(store, args),
        CommandId::Ttl => ttl::ttl(store, args),
        CommandId::PTtl => ttl::pttl(store, args),
        _ => RespFrame::error_static("ERR unknown command"),
    }
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

    let mut items = items.into_iter();
    if let Some(first_item) = items.next() {
        let mut first = parse_arg(first_item)?;
        uppercase_compact_arg_in_place(&mut first);
        args.push(first);

        for item in items {
            args.push(parse_arg(item)?);
        }
    }

    Ok(())
}

#[inline]
fn parse_arg(item: RespFrame) -> Result<CompactArg, &'static str> {
    match item {
        RespFrame::Bulk(Some(BulkData::Arg(bytes))) => Ok(bytes),
        RespFrame::Bulk(Some(BulkData::Value(bytes))) => Ok(CompactArg::from_vec(bytes.into_vec())),
        RespFrame::Simple(value) => Ok(CompactArg::from_vec(value.into_bytes())),
        RespFrame::SimpleStatic(value) => Ok(CompactArg::from_slice(value.as_bytes())),
        _ => Err("ERR invalid argument type"),
    }
}

#[inline]
fn uppercase_compact_arg_in_place(arg: &mut CompactArg) {
    arg.make_ascii_uppercase();
}

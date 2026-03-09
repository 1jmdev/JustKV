use crate::command::CommandId;

#[inline(always)]
fn eq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[inline(always)]
pub fn dispatch_safe(input: &[u8]) -> Option<CommandId> {
    if input.is_empty() {
        return None;
    }

    let f = input[0] | 0x20;

    match input.len() {
        3 => match f {
            b'a' => {
                if eq(input, b"ACL") {
                    return Some(CommandId::Acl);
                }
            }
            b'd' => {
                if eq(input, b"DEL") {
                    return Some(CommandId::Del);
                }
            }
            b'g' => {
                if eq(input, b"GET") {
                    return Some(CommandId::Get);
                }
            }
            b'l' => {
                if eq(input, b"LCS") {
                    return Some(CommandId::Lcs);
                }
            }
            b's' => {
                if eq(input, b"SET") {
                    return Some(CommandId::Set);
                }
            }
            b't' => {
                if eq(input, b"TTL") {
                    return Some(CommandId::Ttl);
                }
            }
            _ => {}
        },

        4 => match f {
            b'a' => {
                if eq(input, b"AUTH") {
                    return Some(CommandId::Auth);
                }
            }
            b'c' => {
                if eq(input, b"COPY") {
                    return Some(CommandId::Copy);
                }
            }
            b'd' => {
                if eq(input, b"DECR") {
                    return Some(CommandId::Decr);
                }
                if eq(input, b"DUMP") {
                    return Some(CommandId::Dump);
                }
            }
            b'e' => {
                if eq(input, b"ECHO") {
                    return Some(CommandId::Echo);
                }
                if eq(input, b"EVAL") {
                    return Some(CommandId::Eval);
                }
                if eq(input, b"EXEC") {
                    return Some(CommandId::Exec);
                }
            }
            b'h' => {
                if eq(input, b"HGET") {
                    return Some(CommandId::HGet);
                }
                if eq(input, b"HDEL") {
                    return Some(CommandId::HDel);
                }
                if eq(input, b"HLEN") {
                    return Some(CommandId::HLen);
                }
                if eq(input, b"HSET") {
                    return Some(CommandId::HSet);
                }
            }
            b'i' => {
                if eq(input, b"INCR") {
                    return Some(CommandId::Incr);
                }
            }
            b'k' => {
                if eq(input, b"KEYS") {
                    return Some(CommandId::Keys);
                }
            }
            b'l' => {
                if eq(input, b"LLEN") {
                    return Some(CommandId::LLen);
                }
                if eq(input, b"LPOS") {
                    return Some(CommandId::LPos);
                }
                if eq(input, b"LREM") {
                    return Some(CommandId::LRem);
                }
                if eq(input, b"LSET") {
                    return Some(CommandId::LSet);
                }
                if eq(input, b"LPOP") {
                    return Some(CommandId::LPop);
                }
            }
            b'm' => {
                if eq(input, b"MGET") {
                    return Some(CommandId::MGet);
                }
                if eq(input, b"MSET") {
                    return Some(CommandId::MSet);
                }
                if eq(input, b"MOVE") {
                    return Some(CommandId::Move);
                }
            }
            b'p' => {
                if eq(input, b"PING") {
                    return Some(CommandId::Ping);
                }
                if eq(input, b"PTTL") {
                    return Some(CommandId::PTtl);
                }
            }
            b'q' => {
                if eq(input, b"QUIT") {
                    return Some(CommandId::Quit);
                }
            }
            b'r' => {
                if eq(input, b"RPOP") {
                    return Some(CommandId::RPop);
                }
            }
            b's' => {
                if eq(input, b"SCAN") {
                    return Some(CommandId::Scan);
                }
                if eq(input, b"SORT") {
                    return Some(CommandId::Sort);
                }
                if eq(input, b"SADD") {
                    return Some(CommandId::SAdd);
                }
                if eq(input, b"SREM") {
                    return Some(CommandId::SRem);
                }
                if eq(input, b"SPOP") {
                    return Some(CommandId::SPop);
                }
            }
            b't' => {
                if eq(input, b"TYPE") {
                    return Some(CommandId::Type);
                }
            }
            b'x' => {
                if eq(input, b"XADD") {
                    return Some(CommandId::XAdd);
                }
                if eq(input, b"XLEN") {
                    return Some(CommandId::XLen);
                }
                if eq(input, b"XDEL") {
                    return Some(CommandId::XDel);
                }
                if eq(input, b"XACK") {
                    return Some(CommandId::XAck);
                }
            }
            b'z' => {
                if eq(input, b"ZADD") {
                    return Some(CommandId::ZAdd);
                }
                if eq(input, b"ZREM") {
                    return Some(CommandId::ZRem);
                }
            }
            _ => {}
        },

        5 => match f {
            b'b' => {
                if eq(input, b"BLPOP") {
                    return Some(CommandId::BLPop);
                }
                if eq(input, b"BRPOP") {
                    return Some(CommandId::BRPop);
                }
                if eq(input, b"BITOP") {
                    return Some(CommandId::BitOp);
                }
            }
            b'd' => {
                if eq(input, b"DELEX") {
                    return Some(CommandId::Delex);
                }
            }
            b'g' => {
                if eq(input, b"GETEX") {
                    return Some(CommandId::GetEx);
                }
            }
            b'h' => {
                if eq(input, b"HELLO") {
                    return Some(CommandId::Hello);
                }
                if eq(input, b"HKEYS") {
                    return Some(CommandId::HKeys);
                }
                if eq(input, b"HVALS") {
                    return Some(CommandId::HVals);
                }
                if eq(input, b"HMGET") {
                    return Some(CommandId::HMGet);
                }
                if eq(input, b"HMSET") {
                    return Some(CommandId::HMSet);
                }
                if eq(input, b"HSCAN") {
                    return Some(CommandId::HScan);
                }
            }
            b'l' => {
                if eq(input, b"LMOVE") {
                    return Some(CommandId::LMove);
                }
                if eq(input, b"LMPOP") {
                    return Some(CommandId::LMPop);
                }
                if eq(input, b"LPUSH") {
                    return Some(CommandId::LPush);
                }
                if eq(input, b"LTRIM") {
                    return Some(CommandId::LTrim);
                }
            }
            b'm' => {
                if eq(input, b"MULTI") {
                    return Some(CommandId::Multi);
                }
            }
            b'p' => {
                if eq(input, b"PFADD") {
                    return Some(CommandId::PfAdd);
                }
            }
            b'r' => {
                if eq(input, b"RPUSH") {
                    return Some(CommandId::RPush);
                }
            }
            b's' => {
                if eq(input, b"SCARD") {
                    return Some(CommandId::SCard);
                }
                if eq(input, b"SMOVE") {
                    return Some(CommandId::SMove);
                }
                if eq(input, b"SSCAN") {
                    return Some(CommandId::SScan);
                }
                if eq(input, b"SDIFF") {
                    return Some(CommandId::SDiff);
                }
                if eq(input, b"SETEX") {
                    return Some(CommandId::SetEx);
                }
                if eq(input, b"SETNX") {
                    return Some(CommandId::SetNx);
                }
            }
            b't' => {
                if eq(input, b"TOUCH") {
                    return Some(CommandId::Touch);
                }
            }
            b'w' => {
                if eq(input, b"WATCH") {
                    return Some(CommandId::Watch);
                }
            }
            b'x' => {
                if eq(input, b"XREAD") {
                    return Some(CommandId::XRead);
                }
                if eq(input, b"XTRIM") {
                    return Some(CommandId::XTrim);
                }
            }
            b'z' => {
                if eq(input, b"ZCARD") {
                    return Some(CommandId::ZCard);
                }
                if eq(input, b"ZDIFF") {
                    return Some(CommandId::ZDiff);
                }
                if eq(input, b"ZMPOP") {
                    return Some(CommandId::ZMPop);
                }
                if eq(input, b"ZRANK") {
                    return Some(CommandId::ZRank);
                }
                if eq(input, b"ZSCAN") {
                    return Some(CommandId::ZScan);
                }
            }
            _ => {}
        },

        6 => match f {
            b'a' => {
                if eq(input, b"APPEND") {
                    return Some(CommandId::Append);
                }
            }
            b'b' => {
                if eq(input, b"BLMPOP") {
                    return Some(CommandId::BLMPop);
                }
                if eq(input, b"BZMPOP") {
                    return Some(CommandId::BZMPop);
                }
                if eq(input, b"BITPOS") {
                    return Some(CommandId::BitPos);
                }
            }
            b'c' => {
                if eq(input, b"CLIENT") {
                    return Some(CommandId::Client);
                }
                if eq(input, b"CONFIG") {
                    return Some(CommandId::Config);
                }
            }
            b'd' => {
                if eq(input, b"DBSIZE") {
                    return Some(CommandId::DbSize);
                }
                if eq(input, b"DECRBY") {
                    return Some(CommandId::DecrBy);
                }
                if eq(input, b"DIGEST") {
                    return Some(CommandId::Digest);
                }
            }
            b'e' => {
                if eq(input, b"EXISTS") {
                    return Some(CommandId::Exists);
                }
                if eq(input, b"EXPIRE") {
                    return Some(CommandId::Expire);
                }
            }
            b'g' => {
                if eq(input, b"GETDEL") {
                    return Some(CommandId::GetDel);
                }
                if eq(input, b"GETSET") {
                    return Some(CommandId::GetSet);
                }
                if eq(input, b"GETBIT") {
                    return Some(CommandId::GetBit);
                }
            }
            b'h' => {
                if eq(input, b"HSETNX") {
                    return Some(CommandId::HSetNx);
                }
            }
            b'i' => {
                if eq(input, b"INCRBY") {
                    return Some(CommandId::IncrBy);
                }
            }
            b'l' => {
                if eq(input, b"LINDEX") {
                    return Some(CommandId::LIndex);
                }
                if eq(input, b"LRANGE") {
                    return Some(CommandId::LRange);
                }
                if eq(input, b"LPUSHX") {
                    return Some(CommandId::LPushX);
                }
            }
            b'm' => {
                if eq(input, b"MSETEX") {
                    return Some(CommandId::MSetEx);
                }
                if eq(input, b"MSETNX") {
                    return Some(CommandId::MSetNx);
                }
            }
            b'p' => {
                if eq(input, b"PSETEX") {
                    return Some(CommandId::PSetEx);
                }
                if eq(input, b"PFCOUNT") {
                    return Some(CommandId::PfCount);
                }
                if eq(input, b"PFMERGE") {
                    return Some(CommandId::PfMerge);
                }
                if eq(input, b"PUBSUB") {
                    return Some(CommandId::PubSub);
                }
                if eq(input, b"PUBLISH") {
                    return Some(CommandId::Publish);
                }
            }
            b'r' => {
                if eq(input, b"RENAME") {
                    return Some(CommandId::Rename);
                }
                if eq(input, b"RPUSHX") {
                    return Some(CommandId::RPushX);
                }
            }
            b's' => {
                if eq(input, b"SELECT") {
                    return Some(CommandId::Select);
                }
                if eq(input, b"SETBIT") {
                    return Some(CommandId::SetBit);
                }
                if eq(input, b"SUBSTR") {
                    return Some(CommandId::SubStr);
                }
                if eq(input, b"SINTER") {
                    return Some(CommandId::SInter);
                }
                if eq(input, b"SUNION") {
                    return Some(CommandId::SUnion);
                }
                if eq(input, b"SCRIPT") {
                    return Some(CommandId::Script);
                }
                if eq(input, b"STRLEN") {
                    return Some(CommandId::StrLen);
                }
            }
            b'u' => {
                if eq(input, b"UNLINK") {
                    return Some(CommandId::Unlink);
                }
            }
            b'x' => {
                if eq(input, b"XRANGE") {
                    return Some(CommandId::XRange);
                }
                if eq(input, b"XGROUP") {
                    return Some(CommandId::XGroup);
                }
                if eq(input, b"XCLAIM") {
                    return Some(CommandId::XClaim);
                }
            }
            b'z' => {
                if eq(input, b"ZCOUNT") {
                    return Some(CommandId::ZCount);
                }
                if eq(input, b"ZINTER") {
                    return Some(CommandId::ZInter);
                }
                if eq(input, b"ZUNION") {
                    return Some(CommandId::ZUnion);
                }
                if eq(input, b"ZRANGE") {
                    return Some(CommandId::ZRange);
                }
                if eq(input, b"ZSCORE") {
                    return Some(CommandId::ZScore);
                }
            }
            _ => {}
        },

        7 => match f {
            b'c' => {
                if eq(input, b"COMMAND") {
                    return Some(CommandId::Command);
                }
            }
            b'd' => {
                if eq(input, b"DISCARD") {
                    return Some(CommandId::Discard);
                }
            }
            b'e' => {
                if eq(input, b"EVAL_RO") {
                    return Some(CommandId::EvalRo);
                }
                if eq(input, b"EVALSHA") {
                    return Some(CommandId::EvalSha);
                }
            }
            b'f' => {
                if eq(input, b"FLUSHDB") {
                    return Some(CommandId::FlushDb);
                }
            }
            b'g' => {
                if eq(input, b"GEOADD") {
                    return Some(CommandId::GeoAdd);
                }
                if eq(input, b"GEOPOS") {
                    return Some(CommandId::GeoPos);
                }
                if eq(input, b"GEOHASH") {
                    return Some(CommandId::GeoHash);
                }
            }
            b'h' => {
                if eq(input, b"HEXISTS") {
                    return Some(CommandId::HExists);
                }
                if eq(input, b"HINCRBY") {
                    return Some(CommandId::HIncrBy);
                }
                if eq(input, b"HGETALL") {
                    return Some(CommandId::HGetAll);
                }
                if eq(input, b"HSTRLEN") {
                    return Some(CommandId::HStrLen);
                }
            }
            b'l' => {
                if eq(input, b"LINSERT") {
                    return Some(CommandId::LInsert);
                }
            }
            b'p' => {
                if eq(input, b"PERSIST") {
                    return Some(CommandId::Persist);
                }
                if eq(input, b"PEXPIRE") {
                    return Some(CommandId::PExpire);
                }
            }
            b'r' => {
                if eq(input, b"RESTORE") {
                    return Some(CommandId::Restore);
                }
            }
            b'u' => {
                if eq(input, b"UNWATCH") {
                    return Some(CommandId::Unwatch);
                }
            }
            b'z' => {
                if eq(input, b"ZINCRBY") {
                    return Some(CommandId::ZIncrBy);
                }
                if eq(input, b"ZMSCORE") {
                    return Some(CommandId::ZMScore);
                }
                if eq(input, b"ZPOPMIN") {
                    return Some(CommandId::ZPopMin);
                }
                if eq(input, b"ZPOPMAX") {
                    return Some(CommandId::ZPopMax);
                }
            }
            _ => {}
        },

        8 => match f {
            b'b' => {
                if eq(input, b"BITCOUNT") {
                    return Some(CommandId::BitCount);
                }
                if eq(input, b"BITFIELD") {
                    return Some(CommandId::BitField);
                }
                if eq(input, b"BZPOPMIN") {
                    return Some(CommandId::BZPopMin);
                }
                if eq(input, b"BZPOPMAX") {
                    return Some(CommandId::BZPopMax);
                }
            }
            b'e' => {
                if eq(input, b"EXPIREAT") {
                    return Some(CommandId::ExpireAt);
                }
            }
            b'f' => {
                if eq(input, b"FLUSHALL") {
                    return Some(CommandId::FlushAll);
                }
            }
            b'g' => {
                if eq(input, b"GETRANGE") {
                    return Some(CommandId::GetRange);
                }
            }
            b'j' => {
                if eq(input, b"JSON.GET") {
                    return Some(CommandId::JsonGet);
                }
                if eq(input, b"JSON.SET") {
                    return Some(CommandId::JsonSet);
                }
                if eq(input, b"JSON.DEL") {
                    return Some(CommandId::JsonDel);
                }
            }
            b'r' => {
                if eq(input, b"RENAMENX") {
                    return Some(CommandId::RenameNx);
                }
            }
            b's' => {
                if eq(input, b"SETRANGE") {
                    return Some(CommandId::SetRange);
                }
                if eq(input, b"SMEMBERS") {
                    return Some(CommandId::SMembers);
                }
            }
            b'x' => {
                if eq(input, b"XPENDING") {
                    return Some(CommandId::XPending);
                }
            }
            b'z' => {
                if eq(input, b"ZREVRANK") {
                    return Some(CommandId::ZRevRank);
                }
            }
            _ => {}
        },

        9 => match f {
            b'g' => {
                if eq(input, b"GEODIST") {
                    return Some(CommandId::GeoDist);
                }
                if eq(input, b"GEOSEARCH") {
                    return Some(CommandId::GeoSearch);
                }
                if eq(input, b"GEORADIUS") {
                    return Some(CommandId::GeoRadius);
                }
            }
            b'j' => {
                if eq(input, b"JSON.MGET") {
                    return Some(CommandId::JsonMGet);
                }
                if eq(input, b"JSON.MSET") {
                    return Some(CommandId::JsonMSet);
                }
                if eq(input, b"JSON.TYPE") {
                    return Some(CommandId::JsonType);
                }
                if eq(input, b"JSON.RESP") {
                    return Some(CommandId::JsonResp);
                }
            }
            b'p' => {
                if eq(input, b"PEXPIREAT") {
                    return Some(CommandId::PExpireAt);
                }
            }
            b'r' => {
                if eq(input, b"RPOPLPUSH") {
                    return Some(CommandId::RPopLPush);
                }
            }
            b's' => {
                if eq(input, b"SISMEMBER") {
                    return Some(CommandId::SIsMember);
                }
                if eq(input, b"SUBSCRIBE") {
                    return Some(CommandId::Subscribe);
                }
            }
            b'x' => {
                if eq(input, b"XREVRANGE") {
                    return Some(CommandId::XRevRange);
                }
            }
            b'z' => {
                if eq(input, b"ZREVRANGE") {
                    return Some(CommandId::ZRevRange);
                }
                if eq(input, b"ZLEXCOUNT") {
                    return Some(CommandId::ZLexCount);
                }
            }
            _ => {}
        },

        10 => match f {
            b'b' => {
                if eq(input, b"BRPOPLPUSH") {
                    return Some(CommandId::BRPopLPush);
                }
            }
            b'e' => {
                if eq(input, b"EVALSHA_RO") {
                    return Some(CommandId::EvalShaRo);
                }
            }
            b'h' => {
                if eq(input, b"HRANDFIELD") {
                    return Some(CommandId::HRandField);
                }
            }
            b'p' => {
                if eq(input, b"PSUBSCRIBE") {
                    return Some(CommandId::PSubscribe);
                }
            }
            b'j' => {
                if eq(input, b"JSON.CLEAR") {
                    return Some(CommandId::JsonClear);
                }
                if eq(input, b"JSON.DEBUG") {
                    return Some(CommandId::JsonDebug);
                }
                if eq(input, b"JSON.MERGE") {
                    return Some(CommandId::JsonMerge);
                }
            }
            b's' => {
                if eq(input, b"SINTERCARD") {
                    return Some(CommandId::SInterCard);
                }
                if eq(input, b"SDIFFSTORE") {
                    return Some(CommandId::SDiffStore);
                }
                if eq(input, b"SMISMEMBER") {
                    return Some(CommandId::SMIsMember);
                }
            }
            b'x' => {
                if eq(input, b"XREADGROUP") {
                    return Some(CommandId::XReadGroup);
                }
                if eq(input, b"XAUTOCLAIM") {
                    return Some(CommandId::XAutoClaim);
                }
            }
            b'z' => {
                if eq(input, b"ZDIFFSTORE") {
                    return Some(CommandId::ZDiffStore);
                }
            }
            _ => {}
        },

        11 => match f {
            b'b' => {
                if eq(input, b"BITFIELD_RO") {
                    return Some(CommandId::BitFieldRo);
                }
            }
            b'i' => {
                if eq(input, b"INCRBYFLOAT") {
                    return Some(CommandId::IncrByFloat);
                }
            }
            b'j' => {
                if eq(input, b"JSON.ARRLEN") {
                    return Some(CommandId::JsonArrLen);
                }
                if eq(input, b"JSON.ARRPOP") {
                    return Some(CommandId::JsonArrPop);
                }
                if eq(input, b"JSON.FORGET") {
                    return Some(CommandId::JsonForget);
                }
                if eq(input, b"JSON.OBJLEN") {
                    return Some(CommandId::JsonObjLen);
                }
                if eq(input, b"JSON.STRLEN") {
                    return Some(CommandId::JsonStrLen);
                }
                if eq(input, b"JSON.TOGGLE") {
                    return Some(CommandId::JsonToggle);
                }
            }
            b's' => {
                if eq(input, b"SINTERSTORE") {
                    return Some(CommandId::SInterStore);
                }
                if eq(input, b"SUNIONSTORE") {
                    return Some(CommandId::SUnionStore);
                }
                if eq(input, b"SRANDMEMBER") {
                    return Some(CommandId::SRandMember);
                }
            }
            b'z' => {
                if eq(input, b"ZRANDMEMBER") {
                    return Some(CommandId::ZRandMember);
                }
                if eq(input, b"ZINTERSTORE") {
                    return Some(CommandId::ZInterStore);
                }
                if eq(input, b"ZUNIONSTORE") {
                    return Some(CommandId::ZUnionStore);
                }
                if eq(input, b"ZRANGESTORE") {
                    return Some(CommandId::ZRangeStore);
                }
                if eq(input, b"ZRANGEBYLEX") {
                    return Some(CommandId::ZRangeByLex);
                }
            }
            b'u' => {
                if eq(input, b"UNSUBSCRIBE") {
                    return Some(CommandId::Unsubscribe);
                }
            }
            _ => {}
        },

        12 => match f {
            b'g' => {
                if eq(input, b"GEORADIUS_RO") {
                    return Some(CommandId::GeoRadiusRo);
                }
            }
            b'h' => {
                if eq(input, b"HINCRBYFLOAT") {
                    return Some(CommandId::HIncrByFloat);
                }
            }
            b'j' => {
                if eq(input, b"JSON.ARRTRIM") {
                    return Some(CommandId::JsonArrTrim);
                }
                if eq(input, b"JSON.OBJKEYS") {
                    return Some(CommandId::JsonObjKeys);
                }
            }
            b'p' => {
                if eq(input, b"PUNSUBSCRIBE") {
                    return Some(CommandId::PUnsubscribe);
                }
            }
            _ => {}
        },

        13 => match f {
            b'j' => {
                if eq(input, b"JSON.ARRINDEX") {
                    return Some(CommandId::JsonArrIndex);
                }
            }
            b'z' => {
                if eq(input, b"ZRANGEBYSCORE") {
                    return Some(CommandId::ZRangeByScore);
                }
            }
            _ => {}
        },

        14 => match f {
            b'g' => {
                if eq(input, b"GEOSEARCHSTORE") {
                    return Some(CommandId::GeoSearchStore);
                }
            }
            b'j' => {
                if eq(input, b"JSON.ARRINSERT") {
                    return Some(CommandId::JsonArrInsert);
                }
                if eq(input, b"JSON.STRAPPEND") {
                    return Some(CommandId::JsonStrAppend);
                }
                if eq(input, b"JSON.ARRAPPEND") {
                    return Some(CommandId::JsonArrAppend);
                }
                if eq(input, b"JSON.NUMINCRBY") {
                    return Some(CommandId::JsonNumIncrBy);
                }
                if eq(input, b"JSON.NUMMULTBY") {
                    return Some(CommandId::JsonNumMultBy);
                }
            }
            b'z' => {
                if eq(input, b"ZREVRANGEBYLEX") {
                    return Some(CommandId::ZRevRangeByLex);
                }
                if eq(input, b"ZREMRANGEBYLEX") {
                    return Some(CommandId::ZRemRangeByLex);
                }
            }
            _ => {}
        },

        15 => {
            if f == b'z' && eq(input, b"ZREMRANGEBYRANK") {
                return Some(CommandId::ZRemRangeByRank);
            }
        }

        16 => {
            if f == b'z' {
                if eq(input, b"ZREVRANGEBYSCORE") {
                    return Some(CommandId::ZRevRangeByScore);
                }
                if eq(input, b"ZREMRANGEBYSCORE") {
                    return Some(CommandId::ZRemRangeByScore);
                }
            }
        }

        17 => {
            if f == b'g' && eq(input, b"GEORADIUSBYMEMBER") {
                return Some(CommandId::GeoRadiusByMember);
            }
        }

        20 => {
            if f == b'g' && eq(input, b"GEORADIUSBYMEMBER_RO") {
                return Some(CommandId::GeoRadiusByMemberRo);
            }
        }

        _ => {}
    }

    None
}

#[inline(always)]
pub fn identify(command: &[u8]) -> CommandId {
    dispatch_safe(command).unwrap_or(CommandId::Unknown)
}

use engine::value::CompactArg;
use protocol::types::RespFrame;

pub type Args = [CompactArg];

/// All known commands parsed from the uppercased command name.
/// The dispatcher matches on this enum to avoid repeated string comparisons
/// on the hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    // Connection
    Auth,
    Hello,
    Client,
    Command,
    Select,
    Quit,
    Ping,
    Echo,
    // Keyspace
    Del,
    Exists,
    Touch,
    Unlink,
    Type,
    Rename,
    Renamenx,
    Dbsize,
    Keys,
    Scan,
    Move,
    Dump,
    Restore,
    Sort,
    Copy,
    Flushdb,
    Flushall,
    // TTL
    Expire,
    Pexpire,
    Expireat,
    Pexpireat,
    Persist,
    Ttl,
    Pttl,
    // String – get/set
    Get,
    Set,
    Setnx,
    Getset,
    Getdel,
    // String – expiry
    Setex,
    Psetex,
    Getex,
    // String – length
    Append,
    Strlen,
    Setrange,
    Getrange,
    // String – multi
    Mget,
    Mset,
    Msetnx,
    // String – counter
    Incr,
    Incrby,
    Decr,
    Decrby,
    // String – bitmap
    Setbit,
    Getbit,
    Bitcount,
    Bitpos,
    Bitop,
    Bitfield,
    BitfieldRo,
    // String – hyperloglog
    Pfadd,
    Pfcount,
    Pfmerge,
    // Hash
    Hset,
    Hmset,
    Hsetnx,
    Hget,
    Hmget,
    Hgetall,
    Hdel,
    Hexists,
    Hkeys,
    Hvals,
    Hlen,
    Hstrlen,
    Hincrby,
    Hincrbyfloat,
    Hrandfield,
    Hscan,
    // List
    Lpush,
    Rpush,
    Lpop,
    Rpop,
    Llen,
    Lindex,
    Lrange,
    Lset,
    Ltrim,
    Linsert,
    Lpos,
    Lmove,
    Brpoplpush,
    Lmpop,
    Blpop,
    Brpop,
    Blmpop,
    // Set
    Sadd,
    Srem,
    Smembers,
    Sismember,
    Scard,
    Smove,
    Spop,
    Srandmember,
    Sinter,
    Sinterstore,
    Sunion,
    Sunionstore,
    Sdiff,
    Sdiffstore,
    Sintercard,
    Sscan,
    // Sorted set
    Zadd,
    Zrem,
    Zcard,
    Zcount,
    Zscore,
    Zrank,
    Zrevrank,
    Zincrby,
    Zmscore,
    Zrange,
    Zrevrange,
    Zrangebyscore,
    Zrevrangebyscore,
    Zpopmin,
    Zpopmax,
    Bzpopmin,
    Bzpopmax,
    Zmpop,
    Bzmpop,
    Zrandmember,
    Zinter,
    Zunion,
    Zdiff,
    Zscan,
    Zremrangebyrank,
    // Geo
    Geoadd,
    Geopos,
    Geodist,
    Geohash,
    Georadius,
    GeoradiusRo,
    Georadiusbymember,
    GeoradiusbymemberRo,
    Geosearch,
    Geosearchstore,
    // Stream
    Xadd,
    Xlen,
    Xdel,
    Xrange,
    Xrevrange,
    Xtrim,
    Xread,
    Xgroup,
    Xreadgroup,
    Xack,
    Xpending,
    Xclaim,
    Xautoclaim,
    // Unknown
    Unknown,
}

pub fn parse_command_id(command: &[u8]) -> CommandId {
    if command.is_empty() {
        return CommandId::Unknown;
    }

    match command.len() {
        3 => match command[0].to_ascii_uppercase() {
            b'D' => {
                if command.eq_ignore_ascii_case(b"DEL") {
                    CommandId::Del
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GET") {
                    CommandId::Get
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SET") {
                    CommandId::Set
                } else {
                    CommandId::Unknown
                }
            }
            b'T' => {
                if command.eq_ignore_ascii_case(b"TTL") {
                    CommandId::Ttl
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        4 => match command[0].to_ascii_uppercase() {
            b'A' => {
                if command.eq_ignore_ascii_case(b"AUTH") {
                    CommandId::Auth
                } else {
                    CommandId::Unknown
                }
            }
            b'C' => {
                if command.eq_ignore_ascii_case(b"COPY") {
                    CommandId::Copy
                } else {
                    CommandId::Unknown
                }
            }
            b'D' => {
                if command.eq_ignore_ascii_case(b"DECR") {
                    CommandId::Decr
                } else if command.eq_ignore_ascii_case(b"DUMP") {
                    CommandId::Dump
                } else {
                    CommandId::Unknown
                }
            }
            b'E' => {
                if command.eq_ignore_ascii_case(b"ECHO") {
                    CommandId::Echo
                } else {
                    CommandId::Unknown
                }
            }
            b'H' => {
                if command.eq_ignore_ascii_case(b"HDEL") {
                    CommandId::Hdel
                } else if command.eq_ignore_ascii_case(b"HGET") {
                    CommandId::Hget
                } else if command.eq_ignore_ascii_case(b"HLEN") {
                    CommandId::Hlen
                } else if command.eq_ignore_ascii_case(b"HSET") {
                    CommandId::Hset
                } else {
                    CommandId::Unknown
                }
            }
            b'I' => {
                if command.eq_ignore_ascii_case(b"INCR") {
                    CommandId::Incr
                } else {
                    CommandId::Unknown
                }
            }
            b'K' => {
                if command.eq_ignore_ascii_case(b"KEYS") {
                    CommandId::Keys
                } else {
                    CommandId::Unknown
                }
            }
            b'L' => {
                if command.eq_ignore_ascii_case(b"LLEN") {
                    CommandId::Llen
                } else if command.eq_ignore_ascii_case(b"LPOP") {
                    CommandId::Lpop
                } else if command.eq_ignore_ascii_case(b"LPOS") {
                    CommandId::Lpos
                } else if command.eq_ignore_ascii_case(b"LSET") {
                    CommandId::Lset
                } else {
                    CommandId::Unknown
                }
            }
            b'M' => {
                if command.eq_ignore_ascii_case(b"MGET") {
                    CommandId::Mget
                } else if command.eq_ignore_ascii_case(b"MOVE") {
                    CommandId::Move
                } else if command.eq_ignore_ascii_case(b"MSET") {
                    CommandId::Mset
                } else {
                    CommandId::Unknown
                }
            }
            b'P' => {
                if command.eq_ignore_ascii_case(b"PING") {
                    CommandId::Ping
                } else if command.eq_ignore_ascii_case(b"PTTL") {
                    CommandId::Pttl
                } else {
                    CommandId::Unknown
                }
            }
            b'Q' => {
                if command.eq_ignore_ascii_case(b"QUIT") {
                    CommandId::Quit
                } else {
                    CommandId::Unknown
                }
            }
            b'R' => {
                if command.eq_ignore_ascii_case(b"RPOP") {
                    CommandId::Rpop
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SADD") {
                    CommandId::Sadd
                } else if command.eq_ignore_ascii_case(b"SCAN") {
                    CommandId::Scan
                } else if command.eq_ignore_ascii_case(b"SORT") {
                    CommandId::Sort
                } else if command.eq_ignore_ascii_case(b"SPOP") {
                    CommandId::Spop
                } else if command.eq_ignore_ascii_case(b"SREM") {
                    CommandId::Srem
                } else {
                    CommandId::Unknown
                }
            }
            b'T' => {
                if command.eq_ignore_ascii_case(b"TYPE") {
                    CommandId::Type
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZADD") {
                    CommandId::Zadd
                } else if command.eq_ignore_ascii_case(b"ZREM") {
                    CommandId::Zrem
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XACK") {
                    CommandId::Xack
                } else if command.eq_ignore_ascii_case(b"XADD") {
                    CommandId::Xadd
                } else if command.eq_ignore_ascii_case(b"XDEL") {
                    CommandId::Xdel
                } else if command.eq_ignore_ascii_case(b"XLEN") {
                    CommandId::Xlen
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        5 => match command[0].to_ascii_uppercase() {
            b'B' => {
                if command.eq_ignore_ascii_case(b"BLPOP") {
                    CommandId::Blpop
                } else if command.eq_ignore_ascii_case(b"BITOP") {
                    CommandId::Bitop
                } else if command.eq_ignore_ascii_case(b"BRPOP") {
                    CommandId::Brpop
                } else {
                    CommandId::Unknown
                }
            }
            b'F' => {
                if command.eq_ignore_ascii_case(b"FLUSH") {
                    CommandId::Flushall
                } else {
                    CommandId::Unknown
                }
            }
            b'P' => {
                if command.eq_ignore_ascii_case(b"PFADD") {
                    CommandId::Pfadd
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GETEX") {
                    CommandId::Getex
                } else {
                    CommandId::Unknown
                }
            }
            b'H' => {
                if command.eq_ignore_ascii_case(b"HELLO") {
                    CommandId::Hello
                } else if command.eq_ignore_ascii_case(b"HKEYS") {
                    CommandId::Hkeys
                } else if command.eq_ignore_ascii_case(b"HMGET") {
                    CommandId::Hmget
                } else if command.eq_ignore_ascii_case(b"HMSET") {
                    CommandId::Hmset
                } else if command.eq_ignore_ascii_case(b"HSCAN") {
                    CommandId::Hscan
                } else if command.eq_ignore_ascii_case(b"HVALS") {
                    CommandId::Hvals
                } else {
                    CommandId::Unknown
                }
            }
            b'L' => {
                if command.eq_ignore_ascii_case(b"LMOVE") {
                    CommandId::Lmove
                } else if command.eq_ignore_ascii_case(b"LMPOP") {
                    CommandId::Lmpop
                } else if command.eq_ignore_ascii_case(b"LPUSH") {
                    CommandId::Lpush
                } else if command.eq_ignore_ascii_case(b"LTRIM") {
                    CommandId::Ltrim
                } else {
                    CommandId::Unknown
                }
            }
            b'R' => {
                if command.eq_ignore_ascii_case(b"RPUSH") {
                    CommandId::Rpush
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SCARD") {
                    CommandId::Scard
                } else if command.eq_ignore_ascii_case(b"SDIFF") {
                    CommandId::Sdiff
                } else if command.eq_ignore_ascii_case(b"SETEX") {
                    CommandId::Setex
                } else if command.eq_ignore_ascii_case(b"SETNX") {
                    CommandId::Setnx
                } else if command.eq_ignore_ascii_case(b"SMOVE") {
                    CommandId::Smove
                } else if command.eq_ignore_ascii_case(b"SSCAN") {
                    CommandId::Sscan
                } else {
                    CommandId::Unknown
                }
            }
            b'T' => {
                if command.eq_ignore_ascii_case(b"TOUCH") {
                    CommandId::Touch
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZCARD") {
                    CommandId::Zcard
                } else if command.eq_ignore_ascii_case(b"ZDIFF") {
                    CommandId::Zdiff
                } else if command.eq_ignore_ascii_case(b"ZMPOP") {
                    CommandId::Zmpop
                } else if command.eq_ignore_ascii_case(b"ZRANK") {
                    CommandId::Zrank
                } else if command.eq_ignore_ascii_case(b"ZSCAN") {
                    CommandId::Zscan
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XREAD") {
                    CommandId::Xread
                } else if command.eq_ignore_ascii_case(b"XTRIM") {
                    CommandId::Xtrim
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        6 => match command[0].to_ascii_uppercase() {
            b'A' => {
                if command.eq_ignore_ascii_case(b"APPEND") {
                    CommandId::Append
                } else {
                    CommandId::Unknown
                }
            }
            b'B' => {
                if command.eq_ignore_ascii_case(b"BLMPOP") {
                    CommandId::Blmpop
                } else if command.eq_ignore_ascii_case(b"BITPOS") {
                    CommandId::Bitpos
                } else if command.eq_ignore_ascii_case(b"BZMPOP") {
                    CommandId::Bzmpop
                } else {
                    CommandId::Unknown
                }
            }
            b'C' => {
                if command.eq_ignore_ascii_case(b"CLIENT") {
                    CommandId::Client
                } else {
                    CommandId::Unknown
                }
            }
            b'D' => {
                if command.eq_ignore_ascii_case(b"DBSIZE") {
                    CommandId::Dbsize
                } else if command.eq_ignore_ascii_case(b"DECRBY") {
                    CommandId::Decrby
                } else {
                    CommandId::Unknown
                }
            }
            b'E' => {
                if command.eq_ignore_ascii_case(b"EXISTS") {
                    CommandId::Exists
                } else if command.eq_ignore_ascii_case(b"EXPIRE") {
                    CommandId::Expire
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GEOADD") {
                    CommandId::Geoadd
                } else if command.eq_ignore_ascii_case(b"GEOPOS") {
                    CommandId::Geopos
                } else if command.eq_ignore_ascii_case(b"GETDEL") {
                    CommandId::Getdel
                } else if command.eq_ignore_ascii_case(b"GETBIT") {
                    CommandId::Getbit
                } else if command.eq_ignore_ascii_case(b"GETSET") {
                    CommandId::Getset
                } else {
                    CommandId::Unknown
                }
            }
            b'H' => {
                if command.eq_ignore_ascii_case(b"HSETNX") {
                    CommandId::Hsetnx
                } else {
                    CommandId::Unknown
                }
            }
            b'I' => {
                if command.eq_ignore_ascii_case(b"INCRBY") {
                    CommandId::Incrby
                } else {
                    CommandId::Unknown
                }
            }
            b'L' => {
                if command.eq_ignore_ascii_case(b"LINDEX") {
                    CommandId::Lindex
                } else if command.eq_ignore_ascii_case(b"LRANGE") {
                    CommandId::Lrange
                } else {
                    CommandId::Unknown
                }
            }
            b'M' => {
                if command.eq_ignore_ascii_case(b"MSETNX") {
                    CommandId::Msetnx
                } else {
                    CommandId::Unknown
                }
            }
            b'P' => {
                if command.eq_ignore_ascii_case(b"PSETEX") {
                    CommandId::Psetex
                } else {
                    CommandId::Unknown
                }
            }
            b'R' => {
                if command.eq_ignore_ascii_case(b"RENAME") {
                    CommandId::Rename
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SELECT") {
                    CommandId::Select
                } else if command.eq_ignore_ascii_case(b"SETBIT") {
                    CommandId::Setbit
                } else if command.eq_ignore_ascii_case(b"SINTER") {
                    CommandId::Sinter
                } else if command.eq_ignore_ascii_case(b"STRLEN") {
                    CommandId::Strlen
                } else if command.eq_ignore_ascii_case(b"SUNION") {
                    CommandId::Sunion
                } else {
                    CommandId::Unknown
                }
            }
            b'U' => {
                if command.eq_ignore_ascii_case(b"UNLINK") {
                    CommandId::Unlink
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZCOUNT") {
                    CommandId::Zcount
                } else if command.eq_ignore_ascii_case(b"ZINTER") {
                    CommandId::Zinter
                } else if command.eq_ignore_ascii_case(b"ZRANGE") {
                    CommandId::Zrange
                } else if command.eq_ignore_ascii_case(b"ZSCORE") {
                    CommandId::Zscore
                } else if command.eq_ignore_ascii_case(b"ZUNION") {
                    CommandId::Zunion
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XCLAIM") {
                    CommandId::Xclaim
                } else if command.eq_ignore_ascii_case(b"XGROUP") {
                    CommandId::Xgroup
                } else if command.eq_ignore_ascii_case(b"XRANGE") {
                    CommandId::Xrange
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        7 => match command[0].to_ascii_uppercase() {
            b'C' => {
                if command.eq_ignore_ascii_case(b"COMMAND") {
                    CommandId::Command
                } else {
                    CommandId::Unknown
                }
            }
            b'F' => {
                if command.eq_ignore_ascii_case(b"FLUSHDB") {
                    CommandId::Flushdb
                } else {
                    CommandId::Unknown
                }
            }
            b'H' => {
                if command.eq_ignore_ascii_case(b"HEXISTS") {
                    CommandId::Hexists
                } else if command.eq_ignore_ascii_case(b"HGETALL") {
                    CommandId::Hgetall
                } else if command.eq_ignore_ascii_case(b"HINCRBY") {
                    CommandId::Hincrby
                } else if command.eq_ignore_ascii_case(b"HSTRLEN") {
                    CommandId::Hstrlen
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GEODIST") {
                    CommandId::Geodist
                } else if command.eq_ignore_ascii_case(b"GEOHASH") {
                    CommandId::Geohash
                } else {
                    CommandId::Unknown
                }
            }
            b'L' => {
                if command.eq_ignore_ascii_case(b"LINSERT") {
                    CommandId::Linsert
                } else {
                    CommandId::Unknown
                }
            }
            b'P' => {
                if command.eq_ignore_ascii_case(b"PERSIST") {
                    CommandId::Persist
                } else if command.eq_ignore_ascii_case(b"PFCOUNT") {
                    CommandId::Pfcount
                } else if command.eq_ignore_ascii_case(b"PFMERGE") {
                    CommandId::Pfmerge
                } else if command.eq_ignore_ascii_case(b"PEXPIRE") {
                    CommandId::Pexpire
                } else {
                    CommandId::Unknown
                }
            }
            b'R' => {
                if command.eq_ignore_ascii_case(b"RESTORE") {
                    CommandId::Restore
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZINCRBY") {
                    CommandId::Zincrby
                } else if command.eq_ignore_ascii_case(b"ZMSCORE") {
                    CommandId::Zmscore
                } else if command.eq_ignore_ascii_case(b"ZPOPMAX") {
                    CommandId::Zpopmax
                } else if command.eq_ignore_ascii_case(b"ZPOPMIN") {
                    CommandId::Zpopmin
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        8 => match command[0].to_ascii_uppercase() {
            b'B' => {
                if command.eq_ignore_ascii_case(b"BITCOUNT") {
                    CommandId::Bitcount
                } else if command.eq_ignore_ascii_case(b"BITFIELD") {
                    CommandId::Bitfield
                } else if command.eq_ignore_ascii_case(b"BZPOPMAX") {
                    CommandId::Bzpopmax
                } else if command.eq_ignore_ascii_case(b"BZPOPMIN") {
                    CommandId::Bzpopmin
                } else {
                    CommandId::Unknown
                }
            }
            b'E' => {
                if command.eq_ignore_ascii_case(b"EXPIREAT") {
                    CommandId::Expireat
                } else {
                    CommandId::Unknown
                }
            }
            b'F' => {
                if command.eq_ignore_ascii_case(b"FLUSHALL") {
                    CommandId::Flushall
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GETRANGE") {
                    CommandId::Getrange
                } else {
                    CommandId::Unknown
                }
            }
            b'R' => {
                if command.eq_ignore_ascii_case(b"RENAMENX") {
                    CommandId::Renamenx
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SETRANGE") {
                    CommandId::Setrange
                } else if command.eq_ignore_ascii_case(b"SMEMBERS") {
                    CommandId::Smembers
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZREVRANK") {
                    CommandId::Zrevrank
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XPENDING") {
                    CommandId::Xpending
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        9 => match command[0].to_ascii_uppercase() {
            b'P' => {
                if command.eq_ignore_ascii_case(b"PEXPIREAT") {
                    CommandId::Pexpireat
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GEORADIUS") {
                    CommandId::Georadius
                } else if command.eq_ignore_ascii_case(b"GEOSEARCH") {
                    CommandId::Geosearch
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SISMEMBER") {
                    CommandId::Sismember
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XREVRANGE") {
                    CommandId::Xrevrange
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZREVRANGE") {
                    CommandId::Zrevrange
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        10 => match command[0].to_ascii_uppercase() {
            b'B' => {
                if command.eq_ignore_ascii_case(b"BRPOPLPUSH") {
                    CommandId::Brpoplpush
                } else {
                    CommandId::Unknown
                }
            }
            b'H' => {
                if command.eq_ignore_ascii_case(b"HRANDFIELD") {
                    CommandId::Hrandfield
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SDIFFSTORE") {
                    CommandId::Sdiffstore
                } else if command.eq_ignore_ascii_case(b"SINTERCARD") {
                    CommandId::Sintercard
                } else {
                    CommandId::Unknown
                }
            }
            b'X' => {
                if command.eq_ignore_ascii_case(b"XAUTOCLAIM") {
                    CommandId::Xautoclaim
                } else if command.eq_ignore_ascii_case(b"XREADGROUP") {
                    CommandId::Xreadgroup
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        11 => match command[0].to_ascii_uppercase() {
            b'B' => {
                if command.eq_ignore_ascii_case(b"BITFIELD_RO") {
                    CommandId::BitfieldRo
                } else {
                    CommandId::Unknown
                }
            }
            b'G' => {
                if command.eq_ignore_ascii_case(b"GEORADIUS_RO") {
                    CommandId::GeoradiusRo
                } else {
                    CommandId::Unknown
                }
            }
            b'S' => {
                if command.eq_ignore_ascii_case(b"SINTERSTORE") {
                    CommandId::Sinterstore
                } else if command.eq_ignore_ascii_case(b"SRANDMEMBER") {
                    CommandId::Srandmember
                } else if command.eq_ignore_ascii_case(b"SUNIONSTORE") {
                    CommandId::Sunionstore
                } else {
                    CommandId::Unknown
                }
            }
            b'Z' => {
                if command.eq_ignore_ascii_case(b"ZRANDMEMBER") {
                    CommandId::Zrandmember
                } else {
                    CommandId::Unknown
                }
            }
            _ => CommandId::Unknown,
        },
        12 => {
            if command.eq_ignore_ascii_case(b"HINCRBYFLOAT") {
                CommandId::Hincrbyfloat
            } else {
                CommandId::Unknown
            }
        }
        13 => {
            if command.eq_ignore_ascii_case(b"ZRANGEBYSCORE") {
                CommandId::Zrangebyscore
            } else {
                CommandId::Unknown
            }
        }
        14 => {
            if command.eq_ignore_ascii_case(b"GEOSEARCHSTORE") {
                CommandId::Geosearchstore
            } else {
                CommandId::Unknown
            }
        }
        15 => {
            if command.eq_ignore_ascii_case(b"GEORADIUSBYMEMBER") {
                CommandId::Georadiusbymember
            } else if command.eq_ignore_ascii_case(b"ZREMRANGEBYRANK") {
                CommandId::Zremrangebyrank
            } else {
                CommandId::Unknown
            }
        }
        16 => {
            if command.eq_ignore_ascii_case(b"ZREVRANGEBYSCORE") {
                CommandId::Zrevrangebyscore
            } else {
                CommandId::Unknown
            }
        }
        18 => {
            if command.eq_ignore_ascii_case(b"GEORADIUSBYMEMBER_RO") {
                CommandId::GeoradiusbymemberRo
            } else {
                CommandId::Unknown
            }
        }
        _ => CommandId::Unknown,
    }
}

pub fn eq_ascii(command: &[u8], expected: &[u8]) -> bool {
    command == expected || command.eq_ignore_ascii_case(expected)
}

pub fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub fn int_error() -> RespFrame {
    RespFrame::error_static("ERR value is not an integer or out of range")
}

pub fn wrong_type() -> RespFrame {
    RespFrame::error_static("WRONGTYPE Operation against a key holding the wrong kind of value")
}

pub fn u64_to_bytes(value: u64) -> Vec<u8> {
    let mut buffer = itoa::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

pub fn f64_to_bytes(value: f64) -> Vec<u8> {
    let mut buffer = ryu::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

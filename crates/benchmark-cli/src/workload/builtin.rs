use super::{BenchKind, BenchSpec};

const PING: &[&[u8]] = &[b"PING"];
const SET: &[&[u8]] = &[b"SET", b"__key__", b"__data__"];
const GET: &[&[u8]] = &[b"GET", b"__key__"];
const INCR: &[&[u8]] = &[b"INCR", b"__key__"];
const DECR: &[&[u8]] = &[b"DECR", b"__key__"];
const INCRBY: &[&[u8]] = &[b"INCRBY", b"__key__", b"1"];
const DECRBY: &[&[u8]] = &[b"DECRBY", b"__key__", b"1"];
const APPEND: &[&[u8]] = &[b"APPEND", b"__key__", b"__data__"];
const GETDEL: &[&[u8]] = &[b"GETDEL", b"__key__"];
const GETEX: &[&[u8]] = &[b"GETEX", b"__key__"];
const SETNX: &[&[u8]] = &[b"SETNX", b"__key__", b"__data__"];
const SETEX: &[&[u8]] = &[b"SETEX", b"__key__", b"60", b"__data__"];
const PSETEX: &[&[u8]] = &[b"PSETEX", b"__key__", b"60000", b"__data__"];
const GETSET: &[&[u8]] = &[b"GETSET", b"__key__", b"__data__"];
const STRLEN: &[&[u8]] = &[b"STRLEN", b"__key__"];
const SETRANGE: &[&[u8]] = &[b"SETRANGE", b"__key__", b"0", b"__data__"];
const GETRANGE: &[&[u8]] = &[b"GETRANGE", b"__key__", b"0", b"2"];
const MSET: &[&[u8]] = &[
    b"MSET",
    b"__key__:0",
    b"__data__",
    b"__key__:1",
    b"__data__",
    b"__key__:2",
    b"__data__",
    b"__key__:3",
    b"__data__",
    b"__key__:4",
    b"__data__",
    b"__key__:5",
    b"__data__",
    b"__key__:6",
    b"__data__",
    b"__key__:7",
    b"__data__",
    b"__key__:8",
    b"__data__",
    b"__key__:9",
    b"__data__",
];
const MGET: &[&[u8]] = &[
    b"MGET",
    b"__key__",
    b"__key__:1",
    b"__key__:2",
    b"__key__:3",
];
const DEL: &[&[u8]] = &[b"DEL", b"__key__"];
const EXISTS: &[&[u8]] = &[b"EXISTS", b"__key__"];
const EXPIRE: &[&[u8]] = &[b"EXPIRE", b"__key__", b"60"];
const PEXPIRE: &[&[u8]] = &[b"PEXPIRE", b"__key__", b"60000"];
const TTL: &[&[u8]] = &[b"TTL", b"__key__"];
const PTTL: &[&[u8]] = &[b"PTTL", b"__key__"];
const PERSIST: &[&[u8]] = &[b"PERSIST", b"__key__"];
const TYPE: &[&[u8]] = &[b"TYPE", b"__key__"];
const TOUCH: &[&[u8]] = &[b"TOUCH", b"__key__"];
const UNLINK: &[&[u8]] = &[b"UNLINK", b"__key__"];
const RENAME: &[&[u8]] = &[b"RENAME", b"__key__", b"__key__:dst"];
const RENAMENX: &[&[u8]] = &[b"RENAMENX", b"__key__", b"__key__:dst"];
const COPY: &[&[u8]] = &[b"COPY", b"__key__", b"__key__:copy"];
const DUMP: &[&[u8]] = &[b"DUMP", b"__key__"];
const RESTORE: &[&[u8]] = &[b"RESTORE", b"__key__:restored", b"0", b"serialized-value"];
const LPUSH: &[&[u8]] = &[b"LPUSH", b"__key__", b"__data__"];
const RPUSH: &[&[u8]] = &[b"RPUSH", b"__key__", b"__data__"];
const LPUSHX: &[&[u8]] = &[b"LPUSHX", b"__key__", b"__data__"];
const RPUSHX: &[&[u8]] = &[b"RPUSHX", b"__key__", b"__data__"];
const LPOP: &[&[u8]] = &[b"LPOP", b"__key__"];
const RPOP: &[&[u8]] = &[b"RPOP", b"__key__"];
const LLEN: &[&[u8]] = &[b"LLEN", b"__key__"];
const LINDEX: &[&[u8]] = &[b"LINDEX", b"__key__", b"0"];
const LSET: &[&[u8]] = &[b"LSET", b"__key__", b"0", b"__data__"];
const LINSERT: &[&[u8]] = &[b"LINSERT", b"__key__", b"BEFORE", b"pivot", b"__data__"];
const LREM: &[&[u8]] = &[b"LREM", b"__key__", b"1", b"__data__"];
const LTRIM: &[&[u8]] = &[b"LTRIM", b"__key__", b"0", b"99"];
const LMOVE: &[&[u8]] = &[b"LMOVE", b"__key__", b"__key__:other", b"LEFT", b"RIGHT"];
const RPOPLPUSH: &[&[u8]] = &[b"RPOPLPUSH", b"__key__", b"__key__:other"];
const LRANGE_100: &[&[u8]] = &[b"LRANGE", b"__key__", b"0", b"99"];
const LRANGE_300: &[&[u8]] = &[b"LRANGE", b"__key__", b"0", b"299"];
const LRANGE_500: &[&[u8]] = &[b"LRANGE", b"__key__", b"0", b"499"];
const LRANGE_600: &[&[u8]] = &[b"LRANGE", b"__key__", b"0", b"599"];
const SADD: &[&[u8]] = &[b"SADD", b"__key__", b"__data__"];
const SREM: &[&[u8]] = &[b"SREM", b"__key__", b"__data__"];
const SISMEMBER: &[&[u8]] = &[b"SISMEMBER", b"__key__", b"__data__"];
const SMISMEMBER: &[&[u8]] = &[b"SMISMEMBER", b"__key__", b"__data__", b"member:2"];
const SCARD: &[&[u8]] = &[b"SCARD", b"__key__"];
const SPOP: &[&[u8]] = &[b"SPOP", b"__key__"];
const SRANDMEMBER: &[&[u8]] = &[b"SRANDMEMBER", b"__key__"];
const SMEMBERS: &[&[u8]] = &[b"SMEMBERS", b"__key__"];
const SUNION: &[&[u8]] = &[b"SUNION", b"__key__", b"__key__:other"];
const SDIFF: &[&[u8]] = &[b"SDIFF", b"__key__", b"__key__:other"];
const SINTER: &[&[u8]] = &[b"SINTER", b"__key__", b"__key__:other"];
const SUNIONSTORE: &[&[u8]] = &[
    b"SUNIONSTORE",
    b"__key__:dest",
    b"__key__",
    b"__key__:other",
];
const SDIFFSTORE: &[&[u8]] = &[b"SDIFFSTORE", b"__key__:dest", b"__key__", b"__key__:other"];
const SINTERSTORE: &[&[u8]] = &[
    b"SINTERSTORE",
    b"__key__:dest",
    b"__key__",
    b"__key__:other",
];
const HSET: &[&[u8]] = &[b"HSET", b"__key__", b"field", b"__data__"];
const HSETNX: &[&[u8]] = &[b"HSETNX", b"__key__", b"field", b"__data__"];
const HGET: &[&[u8]] = &[b"HGET", b"__key__", b"field"];
const HMGET: &[&[u8]] = &[b"HMGET", b"__key__", b"field", b"field:2"];
const HMSET: &[&[u8]] = &[
    b"HMSET",
    b"__key__",
    b"field",
    b"__data__",
    b"field:2",
    b"__data__",
];
const HGETALL: &[&[u8]] = &[b"HGETALL", b"__key__"];
const HDEL: &[&[u8]] = &[b"HDEL", b"__key__", b"field"];
const HEXISTS: &[&[u8]] = &[b"HEXISTS", b"__key__", b"field"];
const HLEN: &[&[u8]] = &[b"HLEN", b"__key__"];
const HINCRBY: &[&[u8]] = &[b"HINCRBY", b"__key__", b"field", b"1"];
const HSTRLEN: &[&[u8]] = &[b"HSTRLEN", b"__key__", b"field"];
const HKEYS: &[&[u8]] = &[b"HKEYS", b"__key__"];
const HVALS: &[&[u8]] = &[b"HVALS", b"__key__"];
const HRANDFIELD: &[&[u8]] = &[b"HRANDFIELD", b"__key__"];
const ZADD: &[&[u8]] = &[b"ZADD", b"__key__", b"1", b"__data__"];
const ZREM: &[&[u8]] = &[b"ZREM", b"__key__", b"__data__"];
const ZCARD: &[&[u8]] = &[b"ZCARD", b"__key__"];
const ZSCORE: &[&[u8]] = &[b"ZSCORE", b"__key__", b"__data__"];
const ZCOUNT: &[&[u8]] = &[b"ZCOUNT", b"__key__", b"-inf", b"+inf"];
const ZRANK: &[&[u8]] = &[b"ZRANK", b"__key__", b"__data__"];
const ZREVRANK: &[&[u8]] = &[b"ZREVRANK", b"__key__", b"__data__"];
const ZRANGE: &[&[u8]] = &[b"ZRANGE", b"__key__", b"0", b"9"];
const ZREVRANGE: &[&[u8]] = &[b"ZREVRANGE", b"__key__", b"0", b"9"];
const ZRANGEBYSCORE: &[&[u8]] = &[
    b"ZRANGEBYSCORE",
    b"__key__",
    b"-inf",
    b"+inf",
    b"LIMIT",
    b"0",
    b"10",
];
const ZREVRANGEBYSCORE: &[&[u8]] = &[
    b"ZREVRANGEBYSCORE",
    b"__key__",
    b"+inf",
    b"-inf",
    b"LIMIT",
    b"0",
    b"10",
];
const ZRANGEBYLEX: &[&[u8]] = &[b"ZRANGEBYLEX", b"__key__", b"-", b"+"];
const ZREVRANGEBYLEX: &[&[u8]] = &[b"ZREVRANGEBYLEX", b"__key__", b"+", b"-"];
const ZLEXCOUNT: &[&[u8]] = &[b"ZLEXCOUNT", b"__key__", b"-", b"+"];
const ZPOPMIN: &[&[u8]] = &[b"ZPOPMIN", b"__key__"];
const ZPOPMAX: &[&[u8]] = &[b"ZPOPMAX", b"__key__"];
const ZINCRBY: &[&[u8]] = &[b"ZINCRBY", b"__key__", b"1", b"__data__"];
const ZREMRANGEBYRANK: &[&[u8]] = &[b"ZREMRANGEBYRANK", b"__key__", b"0", b"9"];
const ZREMRANGEBYSCORE: &[&[u8]] = &[b"ZREMRANGEBYSCORE", b"__key__", b"-inf", b"+inf"];
const PFADD: &[&[u8]] = &[b"PFADD", b"__key__", b"__data__"];
const PFCOUNT: &[&[u8]] = &[b"PFCOUNT", b"__key__"];
const PFMERGE: &[&[u8]] = &[b"PFMERGE", b"__key__:dest", b"__key__", b"__key__:other"];
const GEOADD: &[&[u8]] = &[
    b"GEOADD",
    b"__key__",
    b"13.361389",
    b"38.115556",
    b"__data__",
];
const GEOPOS: &[&[u8]] = &[b"GEOPOS", b"__key__", b"__data__"];
const GEODIST: &[&[u8]] = &[b"GEODIST", b"__key__", b"Palermo", b"Catania", b"km"];
const GEOHASH: &[&[u8]] = &[b"GEOHASH", b"__key__", b"__data__"];
const GEORADIUS: &[&[u8]] = &[b"GEORADIUS", b"__key__", b"15", b"37", b"200", b"km"];
const GEOSEARCH: &[&[u8]] = &[
    b"GEOSEARCH",
    b"__key__",
    b"FROMLONLAT",
    b"15",
    b"37",
    b"BYRADIUS",
    b"200",
    b"km",
];
const XADD: &[&[u8]] = &[b"XADD", b"__key__", b"*", b"field", b"__data__"];
const XLEN: &[&[u8]] = &[b"XLEN", b"__key__"];
const XRANGE: &[&[u8]] = &[b"XRANGE", b"__key__", b"-", b"+"];
const XREVRANGE: &[&[u8]] = &[b"XREVRANGE", b"__key__", b"+", b"-"];
const XDEL: &[&[u8]] = &[b"XDEL", b"__key__", b"0-1"];
const XTRIM: &[&[u8]] = &[b"XTRIM", b"__key__", b"MAXLEN", b"~", b"1000"];
const XPENDING: &[&[u8]] = &[b"XPENDING", b"__key__", b"group"];
const XACK: &[&[u8]] = &[b"XACK", b"__key__", b"group", b"0-1"];
const EVAL: &[&[u8]] = &[
    b"EVAL",
    b"return redis.call('GET', KEYS[1])",
    b"1",
    b"__key__",
];
const EVALSHA: &[&[u8]] = &[
    b"EVALSHA",
    b"ffffffffffffffffffffffffffffffffffffffff",
    b"1",
    b"__key__",
];
const BITCOUNT: &[&[u8]] = &[b"BITCOUNT", b"__key__"];
const BITPOS: &[&[u8]] = &[b"BITPOS", b"__key__", b"1"];
const GETBIT: &[&[u8]] = &[b"GETBIT", b"__key__", b"0"];
const SETBIT: &[&[u8]] = &[b"SETBIT", b"__key__", b"0", b"1"];
const BITFIELD: &[&[u8]] = &[b"BITFIELD", b"__key__", b"GET", b"u4", b"0"];
const BITOP_AND: &[&[u8]] = &[
    b"BITOP",
    b"AND",
    b"__key__:dest",
    b"__key__",
    b"__key__:other",
];
const BITOP_OR: &[&[u8]] = &[
    b"BITOP",
    b"OR",
    b"__key__:dest",
    b"__key__",
    b"__key__:other",
];
const BITOP_XOR: &[&[u8]] = &[
    b"BITOP",
    b"XOR",
    b"__key__:dest",
    b"__key__",
    b"__key__:other",
];
const BITOP_NOT: &[&[u8]] = &[b"BITOP", b"NOT", b"__key__:dest", b"__key__"];
const WATCH: &[&[u8]] = &[b"WATCH", b"__key__"];
const UNWATCH: &[&[u8]] = &[b"UNWATCH"];
const MULTI: &[&[u8]] = &[b"MULTI"];
const DISCARD: &[&[u8]] = &[b"DISCARD"];
const EXEC: &[&[u8]] = &[b"EXEC"];
const PUBLISH: &[&[u8]] = &[b"PUBLISH", b"benchmark-channel", b"__data__"];
const SPUBLISH: &[&[u8]] = &[b"SPUBLISH", b"benchmark-shard-channel", b"__data__"];
const PUBSUB_CHANNELS: &[&[u8]] = &[b"PUBSUB", b"CHANNELS"];
const DBSIZE: &[&[u8]] = &[b"DBSIZE"];
const COMMAND: &[&[u8]] = &[b"COMMAND"];
const COMMAND_COUNT: &[&[u8]] = &[b"COMMAND", b"COUNT"];
const COMMAND_LIST: &[&[u8]] = &[b"COMMAND", b"LIST"];
const COMMAND_INFO: &[&[u8]] = &[b"COMMAND", b"INFO", b"GET"];
const CLIENT_ID: &[&[u8]] = &[b"CLIENT", b"ID"];
const CLIENT_SETINFO: &[&[u8]] = &[b"CLIENT", b"SETINFO", b"LIB-NAME", b"betterkv-benchmark"];
const CLIENT_GETNAME: &[&[u8]] = &[b"CLIENT", b"GETNAME"];
const CLIENT_SETNAME: &[&[u8]] = &[b"CLIENT", b"SETNAME", b"betterkv-benchmark"];
const CLIENT_INFO: &[&[u8]] = &[b"CLIENT", b"INFO"];
const CLIENT_LIST: &[&[u8]] = &[b"CLIENT", b"LIST"];
const CLIENT_PAUSE: &[&[u8]] = &[b"CLIENT", b"PAUSE", b"1"];
const CLIENT_UNPAUSE: &[&[u8]] = &[b"CLIENT", b"UNPAUSE"];
const ECHO: &[&[u8]] = &[b"ECHO", b"__data__"];
const TIME: &[&[u8]] = &[b"TIME"];
const LASTSAVE: &[&[u8]] = &[b"LASTSAVE"];
const INFO: &[&[u8]] = &[b"INFO"];
const MEMORY_USAGE: &[&[u8]] = &[b"MEMORY", b"USAGE", b"__key__"];
const MEMORY_STATS: &[&[u8]] = &[b"MEMORY", b"STATS"];
const MEMORY_MALLOC_STATS: &[&[u8]] = &[b"MEMORY", b"MALLOC-STATS"];
const RANDOMKEY: &[&[u8]] = &[b"RANDOMKEY"];
const SCAN: &[&[u8]] = &[b"SCAN", b"0", b"COUNT", b"10"];
const SSCAN: &[&[u8]] = &[b"SSCAN", b"__key__", b"0", b"COUNT", b"10"];
const HSCAN: &[&[u8]] = &[b"HSCAN", b"__key__", b"0", b"COUNT", b"10"];
const ZSCAN: &[&[u8]] = &[b"ZSCAN", b"__key__", b"0", b"COUNT", b"10"];

pub(crate) const TESTS: &[BenchSpec] = &[
    bench("ping_inline", "PING_INLINE", BenchKind::PingInline, None),
    bench("ping_mbulk", "PING_MBULK", BenchKind::PingMbulk, Some(PING)),
    bench("set", "SET", BenchKind::Set, Some(SET)),
    bench("get", "GET", BenchKind::Get, Some(GET)),
    bench("incr", "INCR", BenchKind::Incr, Some(INCR)),
    bench("decr", "DECR", BenchKind::Custom, Some(DECR)),
    bench("incrby", "INCRBY", BenchKind::Custom, Some(INCRBY)),
    bench("decrby", "DECRBY", BenchKind::Custom, Some(DECRBY)),
    bench("append", "APPEND", BenchKind::Custom, Some(APPEND)),
    bench("getdel", "GETDEL", BenchKind::Custom, Some(GETDEL)),
    bench("getex", "GETEX", BenchKind::Custom, Some(GETEX)),
    bench("setnx", "SETNX", BenchKind::Custom, Some(SETNX)),
    bench("setex", "SETEX", BenchKind::Custom, Some(SETEX)),
    bench("psetex", "PSETEX", BenchKind::Custom, Some(PSETEX)),
    bench("getset", "GETSET", BenchKind::Custom, Some(GETSET)),
    bench("strlen", "STRLEN", BenchKind::Custom, Some(STRLEN)),
    bench("setrange", "SETRANGE", BenchKind::Custom, Some(SETRANGE)),
    bench("getrange", "GETRANGE", BenchKind::Custom, Some(GETRANGE)),
    bench("mset", "MSET", BenchKind::Mset, Some(MSET)),
    bench("mget", "MGET", BenchKind::Custom, Some(MGET)),
    bench("del", "DEL", BenchKind::Custom, Some(DEL)),
    bench("exists", "EXISTS", BenchKind::Custom, Some(EXISTS)),
    bench("expire", "EXPIRE", BenchKind::Custom, Some(EXPIRE)),
    bench("pexpire", "PEXPIRE", BenchKind::Custom, Some(PEXPIRE)),
    bench("ttl", "TTL", BenchKind::Custom, Some(TTL)),
    bench("pttl", "PTTL", BenchKind::Custom, Some(PTTL)),
    bench("persist", "PERSIST", BenchKind::Custom, Some(PERSIST)),
    bench("type", "TYPE", BenchKind::Custom, Some(TYPE)),
    bench("touch", "TOUCH", BenchKind::Custom, Some(TOUCH)),
    bench("unlink", "UNLINK", BenchKind::Custom, Some(UNLINK)),
    bench("rename", "RENAME", BenchKind::Custom, Some(RENAME)),
    bench("renamenx", "RENAMENX", BenchKind::Custom, Some(RENAMENX)),
    bench("copy", "COPY", BenchKind::Custom, Some(COPY)),
    bench("dump", "DUMP", BenchKind::Custom, Some(DUMP)),
    bench("restore", "RESTORE", BenchKind::Custom, Some(RESTORE)),
    bench("lpush", "LPUSH", BenchKind::Lpush, Some(LPUSH)),
    bench("rpush", "RPUSH", BenchKind::Rpush, Some(RPUSH)),
    bench("lpushx", "LPUSHX", BenchKind::Custom, Some(LPUSHX)),
    bench("rpushx", "RPUSHX", BenchKind::Custom, Some(RPUSHX)),
    bench("lpop", "LPOP", BenchKind::Lpop, Some(LPOP)),
    bench("rpop", "RPOP", BenchKind::Rpop, Some(RPOP)),
    bench("llen", "LLEN", BenchKind::Custom, Some(LLEN)),
    bench("lindex", "LINDEX", BenchKind::Custom, Some(LINDEX)),
    bench("lset", "LSET", BenchKind::Custom, Some(LSET)),
    bench("linsert", "LINSERT", BenchKind::Custom, Some(LINSERT)),
    bench("lrem", "LREM", BenchKind::Custom, Some(LREM)),
    bench("ltrim", "LTRIM", BenchKind::Custom, Some(LTRIM)),
    bench("lmove", "LMOVE", BenchKind::Custom, Some(LMOVE)),
    bench("rpoplpush", "RPOPLPUSH", BenchKind::Custom, Some(RPOPLPUSH)),
    bench(
        "lrange_100",
        "LRANGE_100",
        BenchKind::Lrange100,
        Some(LRANGE_100),
    ),
    bench(
        "lrange_300",
        "LRANGE_300",
        BenchKind::Lrange300,
        Some(LRANGE_300),
    ),
    bench(
        "lrange_500",
        "LRANGE_500",
        BenchKind::Lrange500,
        Some(LRANGE_500),
    ),
    bench(
        "lrange_600",
        "LRANGE_600",
        BenchKind::Lrange600,
        Some(LRANGE_600),
    ),
    bench("sadd", "SADD", BenchKind::Sadd, Some(SADD)),
    bench("srem", "SREM", BenchKind::Custom, Some(SREM)),
    bench("sismember", "SISMEMBER", BenchKind::Custom, Some(SISMEMBER)),
    bench(
        "smismember",
        "SMISMEMBER",
        BenchKind::Custom,
        Some(SMISMEMBER),
    ),
    bench("scard", "SCARD", BenchKind::Custom, Some(SCARD)),
    bench("spop", "SPOP", BenchKind::Spop, Some(SPOP)),
    bench(
        "srandmember",
        "SRANDMEMBER",
        BenchKind::Custom,
        Some(SRANDMEMBER),
    ),
    bench("smembers", "SMEMBERS", BenchKind::Custom, Some(SMEMBERS)),
    bench("sunion", "SUNION", BenchKind::Custom, Some(SUNION)),
    bench("sdiff", "SDIFF", BenchKind::Custom, Some(SDIFF)),
    bench("sinter", "SINTER", BenchKind::Custom, Some(SINTER)),
    bench(
        "sunionstore",
        "SUNIONSTORE",
        BenchKind::Custom,
        Some(SUNIONSTORE),
    ),
    bench(
        "sdiffstore",
        "SDIFFSTORE",
        BenchKind::Custom,
        Some(SDIFFSTORE),
    ),
    bench(
        "sinterstore",
        "SINTERSTORE",
        BenchKind::Custom,
        Some(SINTERSTORE),
    ),
    bench("hset", "HSET", BenchKind::Hset, Some(HSET)),
    bench("hsetnx", "HSETNX", BenchKind::Custom, Some(HSETNX)),
    bench("hget", "HGET", BenchKind::Custom, Some(HGET)),
    bench("hmget", "HMGET", BenchKind::Custom, Some(HMGET)),
    bench("hmset", "HMSET", BenchKind::Custom, Some(HMSET)),
    bench("hgetall", "HGETALL", BenchKind::Custom, Some(HGETALL)),
    bench("hdel", "HDEL", BenchKind::Custom, Some(HDEL)),
    bench("hexists", "HEXISTS", BenchKind::Custom, Some(HEXISTS)),
    bench("hlen", "HLEN", BenchKind::Custom, Some(HLEN)),
    bench("hincrby", "HINCRBY", BenchKind::Custom, Some(HINCRBY)),
    bench("hstrlen", "HSTRLEN", BenchKind::Custom, Some(HSTRLEN)),
    bench("hkeys", "HKEYS", BenchKind::Custom, Some(HKEYS)),
    bench("hvals", "HVALS", BenchKind::Custom, Some(HVALS)),
    bench(
        "hrandfield",
        "HRANDFIELD",
        BenchKind::Custom,
        Some(HRANDFIELD),
    ),
    bench("zadd", "ZADD", BenchKind::Zadd, Some(ZADD)),
    bench("zrem", "ZREM", BenchKind::Custom, Some(ZREM)),
    bench("zcard", "ZCARD", BenchKind::Custom, Some(ZCARD)),
    bench("zscore", "ZSCORE", BenchKind::Custom, Some(ZSCORE)),
    bench("zcount", "ZCOUNT", BenchKind::Custom, Some(ZCOUNT)),
    bench("zrank", "ZRANK", BenchKind::Custom, Some(ZRANK)),
    bench("zrevrank", "ZREVRANK", BenchKind::Custom, Some(ZREVRANK)),
    bench("zrange", "ZRANGE", BenchKind::Custom, Some(ZRANGE)),
    bench("zrevrange", "ZREVRANGE", BenchKind::Custom, Some(ZREVRANGE)),
    bench(
        "zrangebyscore",
        "ZRANGEBYSCORE",
        BenchKind::Custom,
        Some(ZRANGEBYSCORE),
    ),
    bench(
        "zrevrangebyscore",
        "ZREVRANGEBYSCORE",
        BenchKind::Custom,
        Some(ZREVRANGEBYSCORE),
    ),
    bench(
        "zrangebylex",
        "ZRANGEBYLEX",
        BenchKind::Custom,
        Some(ZRANGEBYLEX),
    ),
    bench(
        "zrevrangebylex",
        "ZREVRANGEBYLEX",
        BenchKind::Custom,
        Some(ZREVRANGEBYLEX),
    ),
    bench("zlexcount", "ZLEXCOUNT", BenchKind::Custom, Some(ZLEXCOUNT)),
    bench("zpopmin", "ZPOPMIN", BenchKind::ZpopMin, Some(ZPOPMIN)),
    bench("zpopmax", "ZPOPMAX", BenchKind::Custom, Some(ZPOPMAX)),
    bench("zincrby", "ZINCRBY", BenchKind::Custom, Some(ZINCRBY)),
    bench(
        "zremrangebyrank",
        "ZREMRANGEBYRANK",
        BenchKind::Custom,
        Some(ZREMRANGEBYRANK),
    ),
    bench(
        "zremrangebyscore",
        "ZREMRANGEBYSCORE",
        BenchKind::Custom,
        Some(ZREMRANGEBYSCORE),
    ),
    bench("pfadd", "PFADD", BenchKind::Custom, Some(PFADD)),
    bench("pfcount", "PFCOUNT", BenchKind::Custom, Some(PFCOUNT)),
    bench("pfmerge", "PFMERGE", BenchKind::Custom, Some(PFMERGE)),
    bench("geoadd", "GEOADD", BenchKind::Custom, Some(GEOADD)),
    bench("geopos", "GEOPOS", BenchKind::Custom, Some(GEOPOS)),
    bench("geodist", "GEODIST", BenchKind::Custom, Some(GEODIST)),
    bench("geohash", "GEOHASH", BenchKind::Custom, Some(GEOHASH)),
    bench("georadius", "GEORADIUS", BenchKind::Custom, Some(GEORADIUS)),
    bench("geosearch", "GEOSEARCH", BenchKind::Custom, Some(GEOSEARCH)),
    bench("xadd", "XADD", BenchKind::Custom, Some(XADD)),
    bench("xlen", "XLEN", BenchKind::Custom, Some(XLEN)),
    bench("xrange", "XRANGE", BenchKind::Custom, Some(XRANGE)),
    bench("xrevrange", "XREVRANGE", BenchKind::Custom, Some(XREVRANGE)),
    bench("xdel", "XDEL", BenchKind::Custom, Some(XDEL)),
    bench("xtrim", "XTRIM", BenchKind::Custom, Some(XTRIM)),
    bench("xpending", "XPENDING", BenchKind::Custom, Some(XPENDING)),
    bench("xack", "XACK", BenchKind::Custom, Some(XACK)),
    bench("eval", "EVAL", BenchKind::Custom, Some(EVAL)),
    bench("evalsha", "EVALSHA", BenchKind::Custom, Some(EVALSHA)),
    bench("bitcount", "BITCOUNT", BenchKind::Custom, Some(BITCOUNT)),
    bench("bitpos", "BITPOS", BenchKind::Custom, Some(BITPOS)),
    bench("getbit", "GETBIT", BenchKind::Custom, Some(GETBIT)),
    bench("setbit", "SETBIT", BenchKind::Custom, Some(SETBIT)),
    bench("bitfield", "BITFIELD", BenchKind::Custom, Some(BITFIELD)),
    bench("bitop_and", "BITOP_AND", BenchKind::Custom, Some(BITOP_AND)),
    bench("bitop_or", "BITOP_OR", BenchKind::Custom, Some(BITOP_OR)),
    bench("bitop_xor", "BITOP_XOR", BenchKind::Custom, Some(BITOP_XOR)),
    bench("bitop_not", "BITOP_NOT", BenchKind::Custom, Some(BITOP_NOT)),
    bench("watch", "WATCH", BenchKind::Custom, Some(WATCH)),
    bench("unwatch", "UNWATCH", BenchKind::Custom, Some(UNWATCH)),
    bench("multi", "MULTI", BenchKind::Custom, Some(MULTI)),
    bench("discard", "DISCARD", BenchKind::Custom, Some(DISCARD)),
    bench("exec", "EXEC", BenchKind::Custom, Some(EXEC)),
    bench("publish", "PUBLISH", BenchKind::Custom, Some(PUBLISH)),
    bench("spublish", "SPUBLISH", BenchKind::Custom, Some(SPUBLISH)),
    bench(
        "pubsub_channels",
        "PUBSUB_CHANNELS",
        BenchKind::Custom,
        Some(PUBSUB_CHANNELS),
    ),
    bench("dbsize", "DBSIZE", BenchKind::Custom, Some(DBSIZE)),
    bench("command", "COMMAND", BenchKind::Custom, Some(COMMAND)),
    bench(
        "command_count",
        "COMMAND_COUNT",
        BenchKind::Custom,
        Some(COMMAND_COUNT),
    ),
    bench(
        "command_list",
        "COMMAND_LIST",
        BenchKind::Custom,
        Some(COMMAND_LIST),
    ),
    bench(
        "command_info",
        "COMMAND_INFO",
        BenchKind::Custom,
        Some(COMMAND_INFO),
    ),
    bench("client_id", "CLIENT_ID", BenchKind::Custom, Some(CLIENT_ID)),
    bench(
        "client_setinfo",
        "CLIENT_SETINFO",
        BenchKind::Custom,
        Some(CLIENT_SETINFO),
    ),
    bench(
        "client_getname",
        "CLIENT_GETNAME",
        BenchKind::Custom,
        Some(CLIENT_GETNAME),
    ),
    bench(
        "client_setname",
        "CLIENT_SETNAME",
        BenchKind::Custom,
        Some(CLIENT_SETNAME),
    ),
    bench(
        "client_info",
        "CLIENT_INFO",
        BenchKind::Custom,
        Some(CLIENT_INFO),
    ),
    bench(
        "client_list",
        "CLIENT_LIST",
        BenchKind::Custom,
        Some(CLIENT_LIST),
    ),
    bench(
        "client_pause",
        "CLIENT_PAUSE",
        BenchKind::Custom,
        Some(CLIENT_PAUSE),
    ),
    bench(
        "client_unpause",
        "CLIENT_UNPAUSE",
        BenchKind::Custom,
        Some(CLIENT_UNPAUSE),
    ),
    bench("echo", "ECHO", BenchKind::Custom, Some(ECHO)),
    bench("time", "TIME", BenchKind::Custom, Some(TIME)),
    bench("lastsave", "LASTSAVE", BenchKind::Custom, Some(LASTSAVE)),
    bench("info", "INFO", BenchKind::Custom, Some(INFO)),
    bench(
        "memory_usage",
        "MEMORY_USAGE",
        BenchKind::Custom,
        Some(MEMORY_USAGE),
    ),
    bench(
        "memory_stats",
        "MEMORY_STATS",
        BenchKind::Custom,
        Some(MEMORY_STATS),
    ),
    bench(
        "memory_malloc_stats",
        "MEMORY_MALLOC_STATS",
        BenchKind::Custom,
        Some(MEMORY_MALLOC_STATS),
    ),
    bench("randomkey", "RANDOMKEY", BenchKind::Custom, Some(RANDOMKEY)),
    bench("scan", "SCAN", BenchKind::Custom, Some(SCAN)),
    bench("sscan", "SSCAN", BenchKind::Custom, Some(SSCAN)),
    bench("hscan", "HSCAN", BenchKind::Custom, Some(HSCAN)),
    bench("zscan", "ZSCAN", BenchKind::Custom, Some(ZSCAN)),
];

pub fn tests() -> &'static [BenchSpec] {
    TESTS
}

pub(crate) fn find_test(input: &str) -> Option<BenchSpec> {
    let normalized = normalize_name(input);
    TESTS
        .iter()
        .copied()
        .find(|spec| normalize_name(spec.key) == normalized)
}

pub(crate) fn unknown_test_error(raw: &str) -> String {
    let supported = TESTS
        .iter()
        .map(|spec| spec.key)
        .collect::<Vec<_>>()
        .join(",");
    format!("unknown test '{raw}', supported tests include: {supported}")
}

fn normalize_name(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

const fn bench(
    key: &'static str,
    name: &'static str,
    kind: BenchKind,
    template: Option<&'static [&'static [u8]]>,
) -> BenchSpec {
    BenchSpec {
        key,
        name,
        kind,
        template,
    }
}

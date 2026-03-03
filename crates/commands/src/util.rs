use engine::value::CompactArg;
use protocol::types::RespFrame;

pub type Args = [CompactArg];

/// Pack up to 8 bytes into a u64, uppercased. This is a const fn so all
/// known commands become compile-time integer constants.
/// The length is encoded in the top byte so "GET" != "GETX" (no prefix collisions).
#[inline(always)]
pub const fn pack8(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut i = 0;
    while i < bytes.len() && i < 8 {
        let b = bytes[i];
        let upper = if b >= b'a' && b <= b'z' { b - 32 } else { b };
        result |= (upper as u64) << (i * 8);
        i += 1;
    }
    // Encode length in the top byte so "GET" != "GETX" — no prefix collisions.
    result |= (bytes.len() as u64) << 56;
    result
}

/// Pack incoming command bytes into a u64 for matching.
/// Uses branchless SWAR uppercasing — all 8 bytes at once.
/// Returns 0 for empty or >8-byte commands (handled separately).
#[inline(always)]
pub fn pack_runtime(cmd: &[u8]) -> u64 {
    let _trace = profiler::scope("commands::util::pack_runtime");
    if cmd.len() > 8 || cmd.is_empty() {
        return 0;
    }

    let mut buf = [0u8; 8];
    // SAFETY: we checked len <= 8
    unsafe {
        std::ptr::copy_nonoverlapping(cmd.as_ptr(), buf.as_mut_ptr(), cmd.len());
    }

    let mut val = u64::from_le_bytes(buf);

    // Branchless ASCII uppercase via SWAR (SIMD Within A Register):
    // For each byte, if it is in a-z, subtract 32.
    const A: u64 = 0x6161_6161_6161_6161; // b'a' repeated
    const ONES: u64 = 0x0101_0101_0101_0101;
    const Z_BOUND: u64 = ONES.wrapping_mul(256 - 26); // (256-26) repeated
    const CASE_BIT: u64 = 0x2020_2020_2020_2020; // bit 5 of each byte

    let lower_dist = val.wrapping_sub(A);
    // A byte is lowercase iff (lower_dist + (256-26)) does NOT overflow into the
    // high bit of that byte, AND the high bit of lower_dist itself is also 0.
    let is_lower = !lower_dist.wrapping_add(Z_BOUND) & !lower_dist & (ONES << 7);
    // Shift the per-byte high bit down to bit 5 to get the case-flip mask.
    let mask = (is_lower >> 2) & CASE_BIT;
    val ^= mask;

    // Zero out bytes beyond cmd.len(), then encode length in the top byte.
    val &= u64::MAX >> ((8 - cmd.len()) * 8);
    val | ((cmd.len() as u64) << 56)
}

/// All ≤8-byte command constants computed at compile time.
pub mod cmd {
    use super::pack8;

    // ── Connection ────────────────────────────────────────────────────────────
    pub const AUTH: u64 = pack8(b"AUTH");
    pub const HELLO: u64 = pack8(b"HELLO");
    pub const CLIENT: u64 = pack8(b"CLIENT");
    pub const COMMAND: u64 = pack8(b"COMMAND");
    pub const SELECT: u64 = pack8(b"SELECT");
    pub const QUIT: u64 = pack8(b"QUIT");
    pub const PING: u64 = pack8(b"PING");
    pub const ECHO: u64 = pack8(b"ECHO");
    pub const EVAL: u64 = pack8(b"EVAL");
    pub const EVAL_RO: u64 = pack8(b"EVAL_RO");
    pub const EVALSHA: u64 = pack8(b"EVALSHA");
    pub const SCRIPT: u64 = pack8(b"SCRIPT");

    // ── Keyspace ──────────────────────────────────────────────────────────────
    pub const DEL: u64 = pack8(b"DEL");
    pub const EXISTS: u64 = pack8(b"EXISTS");
    pub const TOUCH: u64 = pack8(b"TOUCH");
    pub const UNLINK: u64 = pack8(b"UNLINK");
    pub const TYPE: u64 = pack8(b"TYPE");
    pub const RENAME: u64 = pack8(b"RENAME");
    pub const RENAMENX: u64 = pack8(b"RENAMENX");
    pub const DBSIZE: u64 = pack8(b"DBSIZE");
    pub const KEYS: u64 = pack8(b"KEYS");
    pub const SCAN: u64 = pack8(b"SCAN");
    pub const MOVE: u64 = pack8(b"MOVE");
    pub const DUMP: u64 = pack8(b"DUMP");
    pub const RESTORE: u64 = pack8(b"RESTORE");
    pub const SORT: u64 = pack8(b"SORT");
    pub const COPY: u64 = pack8(b"COPY");
    pub const FLUSHDB: u64 = pack8(b"FLUSHDB");
    pub const FLUSHALL: u64 = pack8(b"FLUSHALL");

    // ── TTL ───────────────────────────────────────────────────────────────────
    pub const EXPIRE: u64 = pack8(b"EXPIRE");
    pub const PEXPIRE: u64 = pack8(b"PEXPIRE");
    pub const EXPIREAT: u64 = pack8(b"EXPIREAT");
    pub const PERSIST: u64 = pack8(b"PERSIST");
    pub const TTL: u64 = pack8(b"TTL");
    pub const PTTL: u64 = pack8(b"PTTL");
    // PEXPIREAT is 9 bytes — handled in dispatch_long

    // ── String ────────────────────────────────────────────────────────────────
    pub const GET: u64 = pack8(b"GET");
    pub const SET: u64 = pack8(b"SET");
    pub const SETNX: u64 = pack8(b"SETNX");
    pub const GETSET: u64 = pack8(b"GETSET");
    pub const GETDEL: u64 = pack8(b"GETDEL");
    pub const SETEX: u64 = pack8(b"SETEX");
    pub const PSETEX: u64 = pack8(b"PSETEX");
    pub const GETEX: u64 = pack8(b"GETEX");
    pub const APPEND: u64 = pack8(b"APPEND");
    pub const STRLEN: u64 = pack8(b"STRLEN");
    pub const SETRANGE: u64 = pack8(b"SETRANGE");
    pub const GETRANGE: u64 = pack8(b"GETRANGE");
    pub const MGET: u64 = pack8(b"MGET");
    pub const MSET: u64 = pack8(b"MSET");
    pub const MSETNX: u64 = pack8(b"MSETNX");
    pub const INCR: u64 = pack8(b"INCR");
    pub const INCRBY: u64 = pack8(b"INCRBY");
    pub const DECR: u64 = pack8(b"DECR");
    pub const DECRBY: u64 = pack8(b"DECRBY");
    pub const SETBIT: u64 = pack8(b"SETBIT");
    pub const GETBIT: u64 = pack8(b"GETBIT");
    pub const BITCOUNT: u64 = pack8(b"BITCOUNT");
    pub const BITPOS: u64 = pack8(b"BITPOS");
    pub const BITOP: u64 = pack8(b"BITOP");
    pub const BITFIELD: u64 = pack8(b"BITFIELD");
    pub const PFADD: u64 = pack8(b"PFADD");
    pub const PFCOUNT: u64 = pack8(b"PFCOUNT");
    pub const PFMERGE: u64 = pack8(b"PFMERGE");

    // ── Hash ──────────────────────────────────────────────────────────────────
    pub const HSET: u64 = pack8(b"HSET");
    pub const HMSET: u64 = pack8(b"HMSET");
    pub const HSETNX: u64 = pack8(b"HSETNX");
    pub const HGET: u64 = pack8(b"HGET");
    pub const HMGET: u64 = pack8(b"HMGET");
    pub const HGETALL: u64 = pack8(b"HGETALL");
    pub const HDEL: u64 = pack8(b"HDEL");
    pub const HEXISTS: u64 = pack8(b"HEXISTS");
    pub const HKEYS: u64 = pack8(b"HKEYS");
    pub const HVALS: u64 = pack8(b"HVALS");
    pub const HLEN: u64 = pack8(b"HLEN");
    pub const HSTRLEN: u64 = pack8(b"HSTRLEN");
    pub const HINCRBY: u64 = pack8(b"HINCRBY");
    pub const HSCAN: u64 = pack8(b"HSCAN");
    // HINCRBYFLOAT is 12 bytes — handled in dispatch_long
    // HRANDFIELD is 10 bytes — handled in dispatch_long

    // ── List ──────────────────────────────────────────────────────────────────
    pub const LPUSH: u64 = pack8(b"LPUSH");
    pub const RPUSH: u64 = pack8(b"RPUSH");
    pub const LPOP: u64 = pack8(b"LPOP");
    pub const RPOP: u64 = pack8(b"RPOP");
    pub const LLEN: u64 = pack8(b"LLEN");
    pub const LINDEX: u64 = pack8(b"LINDEX");
    pub const LRANGE: u64 = pack8(b"LRANGE");
    pub const LSET: u64 = pack8(b"LSET");
    pub const LTRIM: u64 = pack8(b"LTRIM");
    pub const LINSERT: u64 = pack8(b"LINSERT");
    pub const LPOS: u64 = pack8(b"LPOS");
    pub const LMOVE: u64 = pack8(b"LMOVE");
    pub const LMPOP: u64 = pack8(b"LMPOP");
    pub const BLPOP: u64 = pack8(b"BLPOP");
    pub const BRPOP: u64 = pack8(b"BRPOP");
    // BRPOPLPUSH is 10 bytes — handled in dispatch_long
    // BLMPOP is 6 bytes
    pub const BLMPOP: u64 = pack8(b"BLMPOP");

    // ── Set ───────────────────────────────────────────────────────────────────
    pub const SADD: u64 = pack8(b"SADD");
    pub const SREM: u64 = pack8(b"SREM");
    pub const SCARD: u64 = pack8(b"SCARD");
    pub const SMOVE: u64 = pack8(b"SMOVE");
    pub const SPOP: u64 = pack8(b"SPOP");
    pub const SINTER: u64 = pack8(b"SINTER");
    pub const SDIFF: u64 = pack8(b"SDIFF");
    pub const SUNION: u64 = pack8(b"SUNION");
    pub const SSCAN: u64 = pack8(b"SSCAN");
    // SMEMBERS is 8 bytes
    pub const SMEMBERS: u64 = pack8(b"SMEMBERS");
    // SISMEMBER is 9 bytes — handled in dispatch_long
    // SINTERSTORE is 11 bytes — handled in dispatch_long
    // SUNIONSTORE is 11 bytes — handled in dispatch_long
    // SDIFFSTORE is 10 bytes — handled in dispatch_long
    // SINTERCARD is 10 bytes — handled in dispatch_long
    // SRANDMEMBER is 11 bytes — handled in dispatch_long

    // ── Sorted set ────────────────────────────────────────────────────────────
    pub const ZADD: u64 = pack8(b"ZADD");
    pub const ZREM: u64 = pack8(b"ZREM");
    pub const ZCARD: u64 = pack8(b"ZCARD");
    pub const ZCOUNT: u64 = pack8(b"ZCOUNT");
    pub const ZSCORE: u64 = pack8(b"ZSCORE");
    pub const ZRANK: u64 = pack8(b"ZRANK");
    pub const ZINCRBY: u64 = pack8(b"ZINCRBY");
    pub const ZMSCORE: u64 = pack8(b"ZMSCORE");
    pub const ZRANGE: u64 = pack8(b"ZRANGE");
    pub const ZPOPMIN: u64 = pack8(b"ZPOPMIN");
    pub const ZPOPMAX: u64 = pack8(b"ZPOPMAX");
    pub const ZMPOP: u64 = pack8(b"ZMPOP");
    pub const ZINTER: u64 = pack8(b"ZINTER");
    pub const ZUNION: u64 = pack8(b"ZUNION");
    pub const ZDIFF: u64 = pack8(b"ZDIFF");
    pub const ZSCAN: u64 = pack8(b"ZSCAN");
    pub const BZMPOP: u64 = pack8(b"BZMPOP");
    pub const BZPOPMIN: u64 = pack8(b"BZPOPMIN");
    pub const BZPOPMAX: u64 = pack8(b"BZPOPMAX");
    // ZREVRANK is 8 bytes
    pub const ZREVRANK: u64 = pack8(b"ZREVRANK");
    // ZREVRANGE is 9 bytes — handled in dispatch_long
    // ZRANGEBYSCORE is 13 bytes — handled in dispatch_long
    // ZREVRANGEBYSCORE is 16 bytes — handled in dispatch_long
    // ZRANDMEMBER is 11 bytes — handled in dispatch_long
    // ZREMRANGEBYRANK is 15 bytes — handled in dispatch_long

    // ── GEO ───────────────────────────────────────────────────────────────────
    pub const GEOADD: u64 = pack8(b"GEOADD");
    pub const GEOPOS: u64 = pack8(b"GEOPOS");
    pub const GEODIST: u64 = pack8(b"GEODIST");
    pub const GEOHASH: u64 = pack8(b"GEOHASH");
    // GEORADIUS is 9 bytes — handled in dispatch_long
    // GEORADIUS_RO is 12 bytes — handled in dispatch_long
    // GEORADIUSBYMEMBER is 17 bytes — handled in dispatch_long
    // GEORADIUSBYMEMBER_RO is 20 bytes — handled in dispatch_long
    // GEOSEARCH is 9 bytes — handled in dispatch_long
    // GEOSEARCHSTORE is 14 bytes — handled in dispatch_long

    // ── Stream ────────────────────────────────────────────────────────────────
    pub const XADD: u64 = pack8(b"XADD");
    pub const XLEN: u64 = pack8(b"XLEN");
    pub const XDEL: u64 = pack8(b"XDEL");
    pub const XRANGE: u64 = pack8(b"XRANGE");
    pub const XTRIM: u64 = pack8(b"XTRIM");
    pub const XREAD: u64 = pack8(b"XREAD");
    pub const XGROUP: u64 = pack8(b"XGROUP");
    pub const XACK: u64 = pack8(b"XACK");
    pub const XCLAIM: u64 = pack8(b"XCLAIM");
    // XREVRANGE is 9 bytes — handled in dispatch_long
    // XREADGROUP is 10 bytes — handled in dispatch_long
    // XPENDING is 8 bytes
    pub const XPENDING: u64 = pack8(b"XPENDING");
    // XAUTOCLAIM is 10 bytes — handled in dispatch_long
}

pub fn eq_ascii(command: &[u8], expected: &[u8]) -> bool {
    let _trace = profiler::scope("commands::util::eq_ascii");
    command == expected || command.eq_ignore_ascii_case(expected)
}

pub fn wrong_args(command: &str) -> RespFrame {
    let _trace = profiler::scope("commands::util::wrong_args");
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub fn int_error() -> RespFrame {
    let _trace = profiler::scope("commands::util::int_error");
    RespFrame::error_static("ERR value is not an integer or out of range")
}

pub fn wrong_type() -> RespFrame {
    let _trace = profiler::scope("commands::util::wrong_type");
    RespFrame::error_static("WRONGTYPE Operation against a key holding the wrong kind of value")
}

pub fn u64_to_bytes(value: u64) -> Vec<u8> {
    let _trace = profiler::scope("commands::util::u64_to_bytes");
    let mut buffer = itoa::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

pub fn f64_to_bytes(value: f64) -> Vec<u8> {
    let _trace = profiler::scope("commands::util::f64_to_bytes");
    let mut buffer = ryu::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

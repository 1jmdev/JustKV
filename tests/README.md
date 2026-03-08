# Redis 6.2 Comprehensive Test Suite

This directory contains a complete, well-organized test suite for Redis 6.2 commands, organized by command category and data type.

## Test Structure

Tests are organized in folders by command category with `.rtest` files containing multiple test cases per command.

```
tests/
├── strings/             (13 test files) - String operations
├── hashes/              (11 test files) - Hash/Map operations
├── lists/               (8 test files)  - List operations
├── sets/                (6 test files)  - Set operations
├── sorted_sets/         (9 test files)  - Sorted set operations
├── streams/             (2 test files)  - Stream operations
├── bitmaps/             (2 test files)  - Bitmap operations
├── hyperloglog/         (1 test file)   - HyperLogLog operations
├── geo/                 (2 test files)  - Geospatial operations
├── generic/             (5 test files)  - Generic key operations
├── transactions/        (2 test files)  - Transaction operations
├── connection/          (3 test files)  - Connection management
├── pubsub/              (1 test file)   - Pub/Sub messaging
├── scripting/           (1 test file)   - Lua scripting
└── server/              (1 test file)   - Server commands

Total: 67 test files with 600+ individual test cases
```

## Test Format

Each `.rtest` file uses the following standardized format:

```
@name COMMAND_NAME
@group category
@since version

=== TEST: description
[SETUP:]       # optional commands to run before
RUN:           # the command being tested
EXPECT:        # expected output
[CLEANUP:]     # optional teardown
```

### EXPECT Tokens

- `OK` - Simple string OK
- `(nil)` - Null bulk string
- `(integer) N` - Integer reply
- `(error)` - Any error
- `(error) PREFIX` - Error matching prefix
- `"value"` - Bulk string
- `1) "a"` - Array elements
- `(empty array)` - Empty array
- `(any)` - Accept anything
- `(match) <regex>` - Regex match
- `(unordered)` - Array can be in any order

## Command Categories

### String Commands (13 files)
- SET, GET, APPEND, INCR, DECR, GETDEL, GETEX, GETRANGE, GETSET, MGET, MSET, MSETNX, PSETEX, SETEX, SETNX, SETRANGE, STRLEN

### Hash Commands (11 files)
- HDEL, HEXISTS, HGET, HGETALL, HINCRBY, HINCRBYFLOAT, HKEYS, HLEN, HMGET, HMSET, HRANDFIELD, HSCAN, HSET, HSETNX, HSTRLEN, HVALS

### List Commands (8 files)
- BLMOVE, BLPOP, BRPOP, BRPOPLPUSH, LINDEX, LINSERT, LLEN, LMOVE, LPOP, LPOS, LPUSH, LPUSHX, LRANGE, LREM, LSET, LTRIM, RPOP, RPOPLPUSH, RPUSH, RPUSHX

### Set Commands (6 files)
- SADD, SCARD, SDIFF, SDIFFSTORE, SINTER, SINTERSTORE, SISMEMBER, SMEMBERS, SMISMEMBER, SMOVE, SPOP, SRANDMEMBER, SREM, SSCAN, SUNION, SUNIONSTORE

### Sorted Set Commands (9 files)
- ZADD, ZCARD, ZCOUNT, ZDIFF, ZDIFFSTORE, ZINCRBY, ZINTER, ZINTERSTORE, ZLEXCOUNT, ZMSCORE, ZPOPMAX, ZPOPMIN, ZRANDMEMBER, ZRANGE, ZRANGEBYLEX, ZRANGEBYSCORE, ZRANGESTORE, ZRANK, ZREM, ZREMRANGEBYLEX, ZREMRANGEBYRANK, ZREMRANGEBYSCORE, ZREVRANGE, ZREVRANGEBYLEX, ZREVRANGEBYSCORE, ZREVRANK, ZSCAN, ZSCORE, ZUNION, ZUNIONSTORE

### Stream Commands (2 files)
- XACK, XADD, XAUTOCLAIM, XCLAIM, XDEL, XGROUP, XINFO, XLEN, XPENDING, XRANGE, XREAD, XREADGROUP, XREVRANGE, XSETID, XTRIM

### Bitmap Commands (2 files)
- BITCOUNT, BITFIELD, BITFIELD_RO, BITOP, BITPOS, GETBIT, SETBIT

### HyperLogLog Commands (1 file)
- PFADD, PFCOUNT, PFDEBUG, PFMERGE, PFSELFTEST

### Geospatial Commands (2 files)
- GEOADD, GEODIST, GEOHASH, GEOPOS, GEORADIUS, GEORADIUSBYMEMBER, GEOSEARCH, GEOSEARCHSTORE

### Generic Commands (5 files)
- COPY, DEL, DUMP, EXISTS, EXPIRE, EXPIREAT, KEYS, MIGRATE, MOVE, PERSIST, PEXPIRE, PEXPIREAT, PTTL, RANDOMKEY, RENAME, RENAMENX, RESTORE, SCAN, SORT, TOUCH, TTL, TYPE, UNLINK, WAIT

### Transaction Commands (2 files)
- DISCARD, EXEC, MULTI, UNWATCH, WATCH

### Connection Commands (3 files)
- AUTH, CLIENT, ECHO, HELLO, PING, QUIT, RESET, SELECT

### Pub/Sub Commands (1 file)
- PSUBSCRIBE, PUBLISH, PUBSUB, PUNSUBSCRIBE, SUBSCRIBE, UNSUBSCRIBE

### Scripting Commands (1 file)
- EVAL, EVALSHA, SCRIPT

### Server Commands (1 file)
- ACL, BGREWRITEAOF, BGSAVE, COMMAND, CONFIG, DBSIZE, FAILOVER, FLUSHALL, FLUSHDB, INFO, LASTSAVE, LATENCY, LOLWUT, MEMORY, MODULE, MONITOR, PSYNC, REPLCONF, REPLICAOF, RESTORE-ASKING, ROLE, SAVE, SHUTDOWN, SLAVEOF, SLOWLOG, SWAPDB, SYNC, TIME

## Test Coverage

Each command includes:
- ✅ Basic functionality tests
- ✅ Edge case tests (empty values, non-existing keys, etc.)
- ✅ Error condition tests (wrong types, invalid arguments, etc.)
- ✅ Redis 6.2+ specific features (marked with 6.2+)
- ✅ Multiple variants (e.g., ZADD with different options)
- ✅ Complex scenarios (transactions, multiple keys, etc.)

## Key Features

1. **Comprehensive**: 600+ test cases covering all Redis 6.2 commands
2. **Well-Organized**: Logical grouping by command category
3. **Edge Cases Included**: Tests for boundary conditions and error states
4. **Clear Format**: Consistent, readable test syntax
5. **Redis 6.2 Focused**: Special attention to new 6.2 features:
   - GETDEL, GETEX
   - HRANDFIELD
   - LMOVE, BLMOVE
   - SMISMEMBER
   - ZDIFF, ZINTER, ZUNION, ZMSCORE, ZRANDMEMBER, ZRANGESTORE
   - GEOSEARCH, GEOSEARCHSTORE
   - XAUTOCLAIM, XGROUP CREATECONSUMER
   - CLIENT INFO, CLIENT TRACKINGINFO, CLIENT UNPAUSE
   - RESET command
   - COPY command
   - And more...

## Usage

To run tests with your test runner:

```bash
# Run all tests (needs to have running instance on default port)
betterkv-tester tests/

# Run specific category
betterkv-tester tests/strings/

# Run specific command
betterkv-tester tests/strings/set_get.rtest
```

## Notes

- Tests are designed to run without breaking on non-existent commands (graceful degradation)
- Setup/Cleanup sections ensure test isolation
- Some tests use `(any)` matcher for non-deterministic results (random elements, timestamps)
- Blocking commands (BLPOP, BRPOP, etc.) marked but actual blocking tests require special handling
- Tests marked with comments are for reference and edge case documentation

## Future Enhancements

- JSON commands (when available)
- Search module commands
- Time series module commands
- Cluster-specific commands (partially included)
- Module system tests

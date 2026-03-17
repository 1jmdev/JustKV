---
title: Commands Reference — BetterKV
description: Full reference for all 225+ BetterKV commands. Covers strings, hashes, lists, sets, sorted sets, streams, JSON, Lua scripting, pub/sub, transactions, and more.
head:
  - - meta
    - property: og:title
      content: Commands Reference — BetterKV
  - - meta
    - property: og:description
      content: Full reference for all 225+ BetterKV commands. Covers strings, hashes, lists, sets, sorted sets, streams, JSON, Lua scripting, pub/sub, transactions, and more.
  - - meta
    - property: og:url
      content: https://docs.betterkv.com/commands/
  - - meta
    - name: twitter:card
      content: summary_large_image
  - - meta
    - name: twitter:title
      content: Commands Reference — BetterKV
  - - meta
    - name: twitter:description
      content: Full reference for all 225+ BetterKV commands. Covers strings, hashes, lists, sets, sorted sets, streams, JSON, Lua scripting, pub/sub, transactions, and more.
---

# Commands Reference

BetterKV currently exposes 225 registry-backed commands. This index is the landing page for the command surface: use it to scan every command, then jump into the individual page for syntax, parameters, return shape, and examples.

## How To Read These Docs

- Every command now has its own page under `/commands/<command>`.
- The command pages are written for developers integrating BetterKV directly or building tooling on top of Redis-compatible behavior.
- Commands that are declared in the registry but still routed through an unsupported handler are called out explicitly on their page.

## Command Groups

### Connection

[`AUTH`](/commands/auth), [`CLIENT`](/commands/client), [`ECHO`](/commands/echo), [`HELLO`](/commands/hello), [`PING`](/commands/ping), [`QUIT`](/commands/quit), [`SELECT`](/commands/select)

### Geo

[`GEOADD`](/commands/geoadd), [`GEODIST`](/commands/geodist), [`GEOHASH`](/commands/geohash), [`GEOPOS`](/commands/geopos), [`GEORADIUS`](/commands/georadius), [`GEORADIUSBYMEMBER`](/commands/georadiusbymember), [`GEORADIUSBYMEMBER_RO`](/commands/georadiusbymember-ro), [`GEORADIUS_RO`](/commands/georadius-ro), [`GEOSEARCH`](/commands/geosearch), [`GEOSEARCHSTORE`](/commands/geosearchstore)

### Hash

[`HDEL`](/commands/hdel), [`HEXISTS`](/commands/hexists), [`HGET`](/commands/hget), [`HGETALL`](/commands/hgetall), [`HINCRBY`](/commands/hincrby), [`HINCRBYFLOAT`](/commands/hincrbyfloat), [`HKEYS`](/commands/hkeys), [`HLEN`](/commands/hlen), [`HMGET`](/commands/hmget), [`HMSET`](/commands/hmset), [`HRANDFIELD`](/commands/hrandfield), [`HSCAN`](/commands/hscan), [`HSET`](/commands/hset), [`HSETNX`](/commands/hsetnx), [`HSTRLEN`](/commands/hstrlen), [`HVALS`](/commands/hvals)

### Keyspace

[`COPY`](/commands/copy), [`DEL`](/commands/del), [`DUMP`](/commands/dump), [`EXISTS`](/commands/exists), [`EXPIRE`](/commands/expire), [`EXPIREAT`](/commands/expireat), [`JSON.ARRAPPEND`](/commands/json-arrappend), [`JSON.ARRINDEX`](/commands/json-arrindex), [`JSON.ARRINSERT`](/commands/json-arrinsert), [`JSON.ARRLEN`](/commands/json-arrlen), [`JSON.ARRPOP`](/commands/json-arrpop), [`JSON.ARRTRIM`](/commands/json-arrtrim), [`JSON.CLEAR`](/commands/json-clear), [`JSON.DEBUG`](/commands/json-debug), [`JSON.DEL`](/commands/json-del), [`JSON.FORGET`](/commands/json-forget), [`JSON.GET`](/commands/json-get), [`JSON.MERGE`](/commands/json-merge), [`JSON.MGET`](/commands/json-mget), [`JSON.MSET`](/commands/json-mset), [`JSON.NUMINCRBY`](/commands/json-numincrby), [`JSON.NUMMULTBY`](/commands/json-nummultby), [`JSON.OBJKEYS`](/commands/json-objkeys), [`JSON.OBJLEN`](/commands/json-objlen), [`JSON.RESP`](/commands/json-resp), [`JSON.SET`](/commands/json-set), [`JSON.STRAPPEND`](/commands/json-strappend), [`JSON.STRLEN`](/commands/json-strlen), [`JSON.TOGGLE`](/commands/json-toggle), [`JSON.TYPE`](/commands/json-type), [`KEYS`](/commands/keys), [`MOVE`](/commands/move), [`OBJECT`](/commands/object), [`PERSIST`](/commands/persist), [`PEXPIRE`](/commands/pexpire), [`PEXPIREAT`](/commands/pexpireat), [`PTTL`](/commands/pttl), [`RANDOMKEY`](/commands/randomkey), [`RENAME`](/commands/rename), [`RENAMENX`](/commands/renamenx), [`RESTORE`](/commands/restore), [`SCAN`](/commands/scan), [`SORT`](/commands/sort), [`TOUCH`](/commands/touch), [`TTL`](/commands/ttl), [`TYPE`](/commands/type), [`UNLINK`](/commands/unlink)

### List

[`BLMPOP`](/commands/blmpop), [`BLPOP`](/commands/blpop), [`BRPOP`](/commands/brpop), [`BRPOPLPUSH`](/commands/brpoplpush), [`LINDEX`](/commands/lindex), [`LINSERT`](/commands/linsert), [`LLEN`](/commands/llen), [`LMOVE`](/commands/lmove), [`LMPOP`](/commands/lmpop), [`LPOP`](/commands/lpop), [`LPOS`](/commands/lpos), [`LPUSH`](/commands/lpush), [`LPUSHX`](/commands/lpushx), [`LRANGE`](/commands/lrange), [`LREM`](/commands/lrem), [`LSET`](/commands/lset), [`LTRIM`](/commands/ltrim), [`RPOP`](/commands/rpop), [`RPOPLPUSH`](/commands/rpoplpush), [`RPUSH`](/commands/rpush), [`RPUSHX`](/commands/rpushx)

### Pub/Sub

[`PSUBSCRIBE`](/commands/psubscribe), [`PUBLISH`](/commands/publish), [`PUBSUB`](/commands/pubsub), [`PUNSUBSCRIBE`](/commands/punsubscribe), [`SPUBLISH`](/commands/spublish), [`SSUBSCRIBE`](/commands/ssubscribe), [`SUBSCRIBE`](/commands/subscribe), [`SUNSUBSCRIBE`](/commands/sunsubscribe), [`UNSUBSCRIBE`](/commands/unsubscribe)

### Scripting

[`EVAL`](/commands/eval), [`EVALSHA`](/commands/evalsha), [`EVALSHA_RO`](/commands/evalsha-ro), [`EVAL_RO`](/commands/eval-ro), [`SCRIPT`](/commands/script)

### Server

[`COMMAND`](/commands/command), [`CONFIG`](/commands/config), [`DBSIZE`](/commands/dbsize), [`FLUSHALL`](/commands/flushall), [`FLUSHDB`](/commands/flushdb)

### Set

[`SADD`](/commands/sadd), [`SCARD`](/commands/scard), [`SDIFF`](/commands/sdiff), [`SDIFFSTORE`](/commands/sdiffstore), [`SINTER`](/commands/sinter), [`SINTERCARD`](/commands/sintercard), [`SINTERSTORE`](/commands/sinterstore), [`SISMEMBER`](/commands/sismember), [`SMEMBERS`](/commands/smembers), [`SMISMEMBER`](/commands/smismember), [`SMOVE`](/commands/smove), [`SPOP`](/commands/spop), [`SRANDMEMBER`](/commands/srandmember), [`SREM`](/commands/srem), [`SSCAN`](/commands/sscan), [`SUNION`](/commands/sunion), [`SUNIONSTORE`](/commands/sunionstore)

### Sorted Set

[`BZMPOP`](/commands/bzmpop), [`BZPOPMAX`](/commands/bzpopmax), [`BZPOPMIN`](/commands/bzpopmin), [`ZADD`](/commands/zadd), [`ZCARD`](/commands/zcard), [`ZCOUNT`](/commands/zcount), [`ZDIFF`](/commands/zdiff), [`ZDIFFSTORE`](/commands/zdiffstore), [`ZINCRBY`](/commands/zincrby), [`ZINTER`](/commands/zinter), [`ZINTERSTORE`](/commands/zinterstore), [`ZLEXCOUNT`](/commands/zlexcount), [`ZMPOP`](/commands/zmpop), [`ZMSCORE`](/commands/zmscore), [`ZPOPMAX`](/commands/zpopmax), [`ZPOPMIN`](/commands/zpopmin), [`ZRANDMEMBER`](/commands/zrandmember), [`ZRANGE`](/commands/zrange), [`ZRANGEBYLEX`](/commands/zrangebylex), [`ZRANGEBYSCORE`](/commands/zrangebyscore), [`ZRANGESTORE`](/commands/zrangestore), [`ZRANK`](/commands/zrank), [`ZREM`](/commands/zrem), [`ZREMRANGEBYLEX`](/commands/zremrangebylex), [`ZREMRANGEBYRANK`](/commands/zremrangebyrank), [`ZREMRANGEBYSCORE`](/commands/zremrangebyscore), [`ZREVRANGE`](/commands/zrevrange), [`ZREVRANGEBYLEX`](/commands/zrevrangebylex), [`ZREVRANGEBYSCORE`](/commands/zrevrangebyscore), [`ZREVRANK`](/commands/zrevrank), [`ZSCAN`](/commands/zscan), [`ZSCORE`](/commands/zscore), [`ZUNION`](/commands/zunion), [`ZUNIONSTORE`](/commands/zunionstore)

### Stream

[`XACK`](/commands/xack), [`XADD`](/commands/xadd), [`XAUTOCLAIM`](/commands/xautoclaim), [`XCLAIM`](/commands/xclaim), [`XDEL`](/commands/xdel), [`XDELEX`](/commands/xdelex), [`XGROUP`](/commands/xgroup), [`XLEN`](/commands/xlen), [`XPENDING`](/commands/xpending), [`XRANGE`](/commands/xrange), [`XREAD`](/commands/xread), [`XREADGROUP`](/commands/xreadgroup), [`XREVRANGE`](/commands/xrevrange), [`XTRIM`](/commands/xtrim)

### String

[`APPEND`](/commands/append), [`BITCOUNT`](/commands/bitcount), [`BITFIELD`](/commands/bitfield), [`BITFIELD_RO`](/commands/bitfield-ro), [`BITOP`](/commands/bitop), [`BITPOS`](/commands/bitpos), [`DECR`](/commands/decr), [`DECRBY`](/commands/decrby), [`DELEX`](/commands/delex), [`DIGEST`](/commands/digest), [`GET`](/commands/get), [`GETBIT`](/commands/getbit), [`GETDEL`](/commands/getdel), [`GETEX`](/commands/getex), [`GETRANGE`](/commands/getrange), [`GETSET`](/commands/getset), [`INCR`](/commands/incr), [`INCRBY`](/commands/incrby), [`INCRBYFLOAT`](/commands/incrbyfloat), [`LCS`](/commands/lcs), [`MGET`](/commands/mget), [`MSET`](/commands/mset), [`MSETEX`](/commands/msetex), [`MSETNX`](/commands/msetnx), [`PFADD`](/commands/pfadd), [`PFCOUNT`](/commands/pfcount), [`PFMERGE`](/commands/pfmerge), [`PSETEX`](/commands/psetex), [`SET`](/commands/set), [`SETBIT`](/commands/setbit), [`SETEX`](/commands/setex), [`SETNX`](/commands/setnx), [`SETRANGE`](/commands/setrange), [`STRLEN`](/commands/strlen), [`SUBSTR`](/commands/substr)

### Transaction

[`DISCARD`](/commands/discard), [`EXEC`](/commands/exec), [`MULTI`](/commands/multi), [`UNWATCH`](/commands/unwatch), [`WATCH`](/commands/watch)

## Complete Command List

- [`APPEND`](/commands/append) - String
- [`AUTH`](/commands/auth) - Connection
- [`BITCOUNT`](/commands/bitcount) - String
- [`BITFIELD`](/commands/bitfield) - String
- [`BITFIELD_RO`](/commands/bitfield-ro) - String
- [`BITOP`](/commands/bitop) - String
- [`BITPOS`](/commands/bitpos) - String
- [`BLMPOP`](/commands/blmpop) - List
- [`BLPOP`](/commands/blpop) - List
- [`BRPOP`](/commands/brpop) - List
- [`BRPOPLPUSH`](/commands/brpoplpush) - List
- [`BZMPOP`](/commands/bzmpop) - Sorted Set
- [`BZPOPMAX`](/commands/bzpopmax) - Sorted Set
- [`BZPOPMIN`](/commands/bzpopmin) - Sorted Set
- [`CLIENT`](/commands/client) - Connection
- [`COMMAND`](/commands/command) - Server
- [`CONFIG`](/commands/config) - Server
- [`COPY`](/commands/copy) - Keyspace
- [`DBSIZE`](/commands/dbsize) - Server
- [`DECR`](/commands/decr) - String
- [`DECRBY`](/commands/decrby) - String
- [`DEL`](/commands/del) - Keyspace
- [`DELEX`](/commands/delex) - String
- [`DIGEST`](/commands/digest) - String
- [`DISCARD`](/commands/discard) - Transaction
- [`DUMP`](/commands/dump) - Keyspace
- [`ECHO`](/commands/echo) - Connection
- [`EVAL`](/commands/eval) - Scripting
- [`EVALSHA`](/commands/evalsha) - Scripting
- [`EVALSHA_RO`](/commands/evalsha-ro) - Scripting
- [`EVAL_RO`](/commands/eval-ro) - Scripting
- [`EXEC`](/commands/exec) - Transaction
- [`EXISTS`](/commands/exists) - Keyspace
- [`EXPIRE`](/commands/expire) - Keyspace
- [`EXPIREAT`](/commands/expireat) - Keyspace
- [`FLUSHALL`](/commands/flushall) - Server
- [`FLUSHDB`](/commands/flushdb) - Server
- [`GEOADD`](/commands/geoadd) - Geo
- [`GEODIST`](/commands/geodist) - Geo
- [`GEOHASH`](/commands/geohash) - Geo
- [`GEOPOS`](/commands/geopos) - Geo
- [`GEORADIUS`](/commands/georadius) - Geo
- [`GEORADIUSBYMEMBER`](/commands/georadiusbymember) - Geo
- [`GEORADIUSBYMEMBER_RO`](/commands/georadiusbymember-ro) - Geo
- [`GEORADIUS_RO`](/commands/georadius-ro) - Geo
- [`GEOSEARCH`](/commands/geosearch) - Geo
- [`GEOSEARCHSTORE`](/commands/geosearchstore) - Geo
- [`GET`](/commands/get) - String
- [`GETBIT`](/commands/getbit) - String
- [`GETDEL`](/commands/getdel) - String
- [`GETEX`](/commands/getex) - String
- [`GETRANGE`](/commands/getrange) - String
- [`GETSET`](/commands/getset) - String
- [`HDEL`](/commands/hdel) - Hash
- [`HELLO`](/commands/hello) - Connection
- [`HEXISTS`](/commands/hexists) - Hash
- [`HGET`](/commands/hget) - Hash
- [`HGETALL`](/commands/hgetall) - Hash
- [`HINCRBY`](/commands/hincrby) - Hash
- [`HINCRBYFLOAT`](/commands/hincrbyfloat) - Hash
- [`HKEYS`](/commands/hkeys) - Hash
- [`HLEN`](/commands/hlen) - Hash
- [`HMGET`](/commands/hmget) - Hash
- [`HMSET`](/commands/hmset) - Hash
- [`HRANDFIELD`](/commands/hrandfield) - Hash
- [`HSCAN`](/commands/hscan) - Hash
- [`HSET`](/commands/hset) - Hash
- [`HSETNX`](/commands/hsetnx) - Hash
- [`HSTRLEN`](/commands/hstrlen) - Hash
- [`HVALS`](/commands/hvals) - Hash
- [`INCR`](/commands/incr) - String
- [`INCRBY`](/commands/incrby) - String
- [`INCRBYFLOAT`](/commands/incrbyfloat) - String
- [`JSON.ARRAPPEND`](/commands/json-arrappend) - Keyspace
- [`JSON.ARRINDEX`](/commands/json-arrindex) - Keyspace
- [`JSON.ARRINSERT`](/commands/json-arrinsert) - Keyspace
- [`JSON.ARRLEN`](/commands/json-arrlen) - Keyspace
- [`JSON.ARRPOP`](/commands/json-arrpop) - Keyspace
- [`JSON.ARRTRIM`](/commands/json-arrtrim) - Keyspace
- [`JSON.CLEAR`](/commands/json-clear) - Keyspace
- [`JSON.DEBUG`](/commands/json-debug) - Keyspace
- [`JSON.DEL`](/commands/json-del) - Keyspace
- [`JSON.FORGET`](/commands/json-forget) - Keyspace
- [`JSON.GET`](/commands/json-get) - Keyspace
- [`JSON.MERGE`](/commands/json-merge) - Keyspace
- [`JSON.MGET`](/commands/json-mget) - Keyspace
- [`JSON.MSET`](/commands/json-mset) - Keyspace
- [`JSON.NUMINCRBY`](/commands/json-numincrby) - Keyspace
- [`JSON.NUMMULTBY`](/commands/json-nummultby) - Keyspace
- [`JSON.OBJKEYS`](/commands/json-objkeys) - Keyspace
- [`JSON.OBJLEN`](/commands/json-objlen) - Keyspace
- [`JSON.RESP`](/commands/json-resp) - Keyspace
- [`JSON.SET`](/commands/json-set) - Keyspace
- [`JSON.STRAPPEND`](/commands/json-strappend) - Keyspace
- [`JSON.STRLEN`](/commands/json-strlen) - Keyspace
- [`JSON.TOGGLE`](/commands/json-toggle) - Keyspace
- [`JSON.TYPE`](/commands/json-type) - Keyspace
- [`KEYS`](/commands/keys) - Keyspace
- [`LCS`](/commands/lcs) - String
- [`LINDEX`](/commands/lindex) - List
- [`LINSERT`](/commands/linsert) - List
- [`LLEN`](/commands/llen) - List
- [`LMOVE`](/commands/lmove) - List
- [`LMPOP`](/commands/lmpop) - List
- [`LPOP`](/commands/lpop) - List
- [`LPOS`](/commands/lpos) - List
- [`LPUSH`](/commands/lpush) - List
- [`LPUSHX`](/commands/lpushx) - List
- [`LRANGE`](/commands/lrange) - List
- [`LREM`](/commands/lrem) - List
- [`LSET`](/commands/lset) - List
- [`LTRIM`](/commands/ltrim) - List
- [`MGET`](/commands/mget) - String
- [`MOVE`](/commands/move) - Keyspace
- [`MSET`](/commands/mset) - String
- [`MSETEX`](/commands/msetex) - String
- [`MSETNX`](/commands/msetnx) - String
- [`MULTI`](/commands/multi) - Transaction
- [`OBJECT`](/commands/object) - Keyspace
- [`PERSIST`](/commands/persist) - Keyspace
- [`PEXPIRE`](/commands/pexpire) - Keyspace
- [`PEXPIREAT`](/commands/pexpireat) - Keyspace
- [`PFADD`](/commands/pfadd) - String
- [`PFCOUNT`](/commands/pfcount) - String
- [`PFMERGE`](/commands/pfmerge) - String
- [`PING`](/commands/ping) - Connection
- [`PSETEX`](/commands/psetex) - String
- [`PSUBSCRIBE`](/commands/psubscribe) - Pub/Sub
- [`PTTL`](/commands/pttl) - Keyspace
- [`PUBLISH`](/commands/publish) - Pub/Sub
- [`PUBSUB`](/commands/pubsub) - Pub/Sub
- [`PUNSUBSCRIBE`](/commands/punsubscribe) - Pub/Sub
- [`QUIT`](/commands/quit) - Connection
- [`RANDOMKEY`](/commands/randomkey) - Keyspace
- [`RENAME`](/commands/rename) - Keyspace
- [`RENAMENX`](/commands/renamenx) - Keyspace
- [`RESTORE`](/commands/restore) - Keyspace
- [`RPOP`](/commands/rpop) - List
- [`RPOPLPUSH`](/commands/rpoplpush) - List
- [`RPUSH`](/commands/rpush) - List
- [`RPUSHX`](/commands/rpushx) - List
- [`SADD`](/commands/sadd) - Set
- [`SCAN`](/commands/scan) - Keyspace
- [`SCARD`](/commands/scard) - Set
- [`SCRIPT`](/commands/script) - Scripting
- [`SDIFF`](/commands/sdiff) - Set
- [`SDIFFSTORE`](/commands/sdiffstore) - Set
- [`SELECT`](/commands/select) - Connection
- [`SET`](/commands/set) - String
- [`SETBIT`](/commands/setbit) - String
- [`SETEX`](/commands/setex) - String
- [`SETNX`](/commands/setnx) - String
- [`SETRANGE`](/commands/setrange) - String
- [`SINTER`](/commands/sinter) - Set
- [`SINTERCARD`](/commands/sintercard) - Set
- [`SINTERSTORE`](/commands/sinterstore) - Set
- [`SISMEMBER`](/commands/sismember) - Set
- [`SMEMBERS`](/commands/smembers) - Set
- [`SMISMEMBER`](/commands/smismember) - Set
- [`SMOVE`](/commands/smove) - Set
- [`SORT`](/commands/sort) - Keyspace
- [`SPOP`](/commands/spop) - Set
- [`SPUBLISH`](/commands/spublish) - Pub/Sub
- [`SRANDMEMBER`](/commands/srandmember) - Set
- [`SREM`](/commands/srem) - Set
- [`SSCAN`](/commands/sscan) - Set
- [`SSUBSCRIBE`](/commands/ssubscribe) - Pub/Sub
- [`STRLEN`](/commands/strlen) - String
- [`SUBSCRIBE`](/commands/subscribe) - Pub/Sub
- [`SUBSTR`](/commands/substr) - String
- [`SUNION`](/commands/sunion) - Set
- [`SUNIONSTORE`](/commands/sunionstore) - Set
- [`SUNSUBSCRIBE`](/commands/sunsubscribe) - Pub/Sub
- [`TOUCH`](/commands/touch) - Keyspace
- [`TTL`](/commands/ttl) - Keyspace
- [`TYPE`](/commands/type) - Keyspace
- [`UNLINK`](/commands/unlink) - Keyspace
- [`UNSUBSCRIBE`](/commands/unsubscribe) - Pub/Sub
- [`UNWATCH`](/commands/unwatch) - Transaction
- [`WATCH`](/commands/watch) - Transaction
- [`XACK`](/commands/xack) - Stream
- [`XADD`](/commands/xadd) - Stream
- [`XAUTOCLAIM`](/commands/xautoclaim) - Stream
- [`XCLAIM`](/commands/xclaim) - Stream
- [`XDEL`](/commands/xdel) - Stream
- [`XDELEX`](/commands/xdelex) - Stream
- [`XGROUP`](/commands/xgroup) - Stream
- [`XLEN`](/commands/xlen) - Stream
- [`XPENDING`](/commands/xpending) - Stream
- [`XRANGE`](/commands/xrange) - Stream
- [`XREAD`](/commands/xread) - Stream
- [`XREADGROUP`](/commands/xreadgroup) - Stream
- [`XREVRANGE`](/commands/xrevrange) - Stream
- [`XTRIM`](/commands/xtrim) - Stream
- [`ZADD`](/commands/zadd) - Sorted Set
- [`ZCARD`](/commands/zcard) - Sorted Set
- [`ZCOUNT`](/commands/zcount) - Sorted Set
- [`ZDIFF`](/commands/zdiff) - Sorted Set
- [`ZDIFFSTORE`](/commands/zdiffstore) - Sorted Set
- [`ZINCRBY`](/commands/zincrby) - Sorted Set
- [`ZINTER`](/commands/zinter) - Sorted Set
- [`ZINTERSTORE`](/commands/zinterstore) - Sorted Set
- [`ZLEXCOUNT`](/commands/zlexcount) - Sorted Set
- [`ZMPOP`](/commands/zmpop) - Sorted Set
- [`ZMSCORE`](/commands/zmscore) - Sorted Set
- [`ZPOPMAX`](/commands/zpopmax) - Sorted Set
- [`ZPOPMIN`](/commands/zpopmin) - Sorted Set
- [`ZRANDMEMBER`](/commands/zrandmember) - Sorted Set
- [`ZRANGE`](/commands/zrange) - Sorted Set
- [`ZRANGEBYLEX`](/commands/zrangebylex) - Sorted Set
- [`ZRANGEBYSCORE`](/commands/zrangebyscore) - Sorted Set
- [`ZRANGESTORE`](/commands/zrangestore) - Sorted Set
- [`ZRANK`](/commands/zrank) - Sorted Set
- [`ZREM`](/commands/zrem) - Sorted Set
- [`ZREMRANGEBYLEX`](/commands/zremrangebylex) - Sorted Set
- [`ZREMRANGEBYRANK`](/commands/zremrangebyrank) - Sorted Set
- [`ZREMRANGEBYSCORE`](/commands/zremrangebyscore) - Sorted Set
- [`ZREVRANGE`](/commands/zrevrange) - Sorted Set
- [`ZREVRANGEBYLEX`](/commands/zrevrangebylex) - Sorted Set
- [`ZREVRANGEBYSCORE`](/commands/zrevrangebyscore) - Sorted Set
- [`ZREVRANK`](/commands/zrevrank) - Sorted Set
- [`ZSCAN`](/commands/zscan) - Sorted Set
- [`ZSCORE`](/commands/zscore) - Sorted Set
- [`ZUNION`](/commands/zunion) - Sorted Set
- [`ZUNIONSTORE`](/commands/zunionstore) - Sorted Set

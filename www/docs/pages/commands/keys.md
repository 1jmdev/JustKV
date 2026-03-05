# Keys & Expiry Commands

Commands for key lifecycle, scans, movement, and expiry.

## Existence and Type

```
DEL key [key ...]
UNLINK key [key ...]
EXISTS key [key ...]
TOUCH key [key ...]
TYPE key
```

Delete, probe, and inspect keys.

**Complexity:** O(1) to O(N) depending on key count and value size

```bash
EXISTS user:1 user:2
TYPE user:1
TOUCH user:1
UNLINK big:list
```

## Rename / Copy / Move

```
RENAME key newkey
RENAMENX key newkey
COPY source destination [DB db] [REPLACE]
MOVE key db
```

Move key names or values across namespaces/databases.

**Complexity:** O(1) for rename/move metadata, O(N) for value copy

```bash
RENAME cache:user:1 user:1
RENAMENX user:1 user:latest
COPY user:1 user:1:backup
MOVE user:1 1
```

## Scan and Enumerate

```
DBSIZE
KEYS pattern
SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]
```

List keys and count database size.

**Complexity:** O(1) for `DBSIZE`, O(N) for `KEYS`, O(1) per `SCAN` call

```bash
DBSIZE
KEYS user:*
SCAN 0 MATCH "user:*" COUNT 100
```

## Dump / Restore / Sort

```
DUMP key
RESTORE key ttl serialized-value [REPLACE]
SORT key [BY pattern] [LIMIT offset count] [GET pattern [GET pattern ...]] [ASC | DESC] [ALPHA] [STORE destination]
```

Serialize values, restore from serialized payloads, and sort list/set/zset views.

**Complexity:** O(N log N) for sorting, otherwise depends on value size

```bash
DUMP user:1
RESTORE user:1:clone 0 "...serialized..." REPLACE
SORT leaderboard DESC LIMIT 0 10
```

## Expiry and TTL

```
EXPIRE key seconds [NX | XX | GT | LT]
PEXPIRE key milliseconds [NX | XX | GT | LT]
EXPIREAT key unix-time-seconds
PEXPIREAT key unix-time-milliseconds
TTL key
PTTL key
PERSIST key
```

Set and read expiration in seconds or milliseconds.

**Complexity:** O(1)

```bash
EXPIRE session 3600
PEXPIREAT session 1767225600000
TTL session
PERSIST session
```

## Database Flush

```
FLUSHDB [ASYNC | SYNC]
FLUSHALL [ASYNC | SYNC]
```

Delete all keys in current DB (`FLUSHDB`) or all DBs (`FLUSHALL`).

```bash
FLUSHDB ASYNC
FLUSHALL ASYNC
```

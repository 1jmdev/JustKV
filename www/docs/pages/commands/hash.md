# Hash Commands

Hashes store field-value pairs under a single key.

## Write Commands

```
HSET key field value [field value ...]
HMSET key field value [field value ...]
HSETNX key field value
HDEL key field [field ...]
```

Create, update, and delete hash fields.

**Complexity:** O(1) per field

```bash
HSET user:1 name "Alice" email "alice@example.com"
HSETNX user:1 created_at "2026-03-05"
HDEL user:1 email
```

## Read Commands

```
HGET key field
HMGET key field [field ...]
HGETALL key
HEXISTS key field
HKEYS key
HVALS key
HLEN key
HSTRLEN key field
```

Fetch one field, many fields, or metadata from a hash.

**Complexity:** O(1) per field for direct access, O(N) for full scans (`HGETALL`, `HKEYS`, `HVALS`)

```bash
HGET user:1 name
HMGET user:1 name email
HGETALL user:1
HSTRLEN user:1 name
```

## Numeric Fields

```
HINCRBY key field increment
HINCRBYFLOAT key field increment
```

Atomically increment integer or floating-point hash fields.

**Complexity:** O(1)

```bash
HSET stats page:home views 0 rating 4.0
HINCRBY stats page:home 1
HINCRBYFLOAT stats rating 0.25
```

## Iteration and Random Access

```
HSCAN key cursor [MATCH pattern] [COUNT count]
HRANDFIELD key [count [WITHVALUES]]
```

Iterate large hashes incrementally or fetch random field(s).

**Complexity:** O(1) per `HSCAN` call, O(N) total; `HRANDFIELD` is O(N) where N is count

```bash
HSCAN user:1 0 MATCH "na*" COUNT 50
HRANDFIELD user:1
HRANDFIELD user:1 2 WITHVALUES
```

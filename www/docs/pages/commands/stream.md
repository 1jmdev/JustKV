# Stream Commands

Streams are append-only logs with consumer group support.

## Core Stream Commands

```
XADD key [NOMKSTREAM] [MAXLEN | MINID [= | ~] threshold [LIMIT count]] <* | id> field value [field value ...]
XLEN key
XDEL key id [id ...]
XTRIM key MAXLEN | MINID [= | ~] threshold [LIMIT count]
```

Append stream entries and manage stream length.

**Complexity:** O(1) amortized for append, O(N) for delete/trim ranges

```bash
XADD mystream * type order id 1001
XLEN mystream
XDEL mystream 1710000000000-0
XTRIM mystream MAXLEN ~ 10000
```

## Range Reads

```
XRANGE key start end [COUNT count]
XREVRANGE key end start [COUNT count]
```

Read stream entries by ID range.

**Complexity:** O(log N + M)

```bash
XRANGE mystream - + COUNT 50
XREVRANGE mystream + - COUNT 10
```

## Streaming Reads

```
XREAD [COUNT count] [BLOCK milliseconds] STREAMS key [key ...] id [id ...]
XREADGROUP GROUP group consumer [COUNT count] [BLOCK milliseconds] [NOACK] STREAMS key [key ...] id [id ...]
```

Read from one or more streams directly or via consumer groups.

**Complexity:** O(M) per returned batch

```bash
XREAD COUNT 10 STREAMS mystream 0-0
XREAD BLOCK 5000 STREAMS mystream $
XREADGROUP GROUP workers c1 COUNT 10 STREAMS mystream >
```

## Consumer Group Management

```
XGROUP CREATE key group id [MKSTREAM]
XGROUP SETID key group id
XGROUP DESTROY key group
XGROUP CREATECONSUMER key group consumer
XGROUP DELCONSUMER key group consumer
XGROUP HELP
```

Create and manage consumer groups and consumers.

```bash
XGROUP CREATE mystream workers $ MKSTREAM
XGROUP CREATECONSUMER mystream workers c1
XGROUP DELCONSUMER mystream workers c1
```

## Ack / Pending / Claim

```
XACK key group id [id ...]
XPENDING key group [start end count [consumer]]
XCLAIM key group consumer min-idle-time id [id ...] [IDLE ms] [TIME unix-ms] [RETRYCOUNT count] [FORCE] [JUSTID] [LASTID id]
XAUTOCLAIM key group consumer min-idle-time start [COUNT count] [JUSTID]
```

Inspect pending messages and transfer ownership of stale entries.

**Complexity:** Depends on pending list size and requested count

```bash
XACK mystream workers 1710000000000-0
XPENDING mystream workers
XAUTOCLAIM mystream workers c2 60000 0-0 COUNT 100
```

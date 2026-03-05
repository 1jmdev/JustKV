# String Commands

String commands operate on string values (up to 512 MB).

## Core Strings

```
GET key
SET key value [NX | XX] [GET] [EX seconds | PX milliseconds | EXAT timestamp | PXAT ms-timestamp | KEEPTTL]
SETNX key value
GETSET key value
GETDEL key
GETEX key [EX seconds | PX milliseconds | EXAT timestamp | PERSIST]
SETEX key seconds value
PSETEX key milliseconds value
```

Read, write, and atomically update string values.

**Complexity:** O(1)

```bash
SET greeting "Hello"
GET greeting
SET greeting "Hi" GET
GETSET greeting "Hello again"
GETDEL greeting
```

## Multi-Key Strings

```
MGET key [key ...]
MSET key value [key value ...]
MSETNX key value [key value ...]
```

Get or set multiple string keys in a single round trip.

**Complexity:** O(N) where N is number of keys

```bash
MSET user:1:name "Alice" user:2:name "Bob"
MGET user:1:name user:2:name
MSETNX lock:a "1" lock:b "1"
```

## String Mutation

```
APPEND key value
STRLEN key
SETRANGE key offset value
GETRANGE key start end
```

Append, measure, and edit substrings.

**Complexity:** O(1) to O(N) depending on command and range size

```bash
SET msg "hello"
APPEND msg " world"
STRLEN msg
GETRANGE msg 0 4
SETRANGE msg 6 "BetterKV"
```

## Bit Operations

```
SETBIT key offset value
GETBIT key offset
BITCOUNT key [start end [BYTE | BIT]]
BITPOS key bit [start [end [BYTE | BIT]]]
BITOP operation destkey key [key ...]
BITFIELD key [GET type offset] [SET type offset value] [INCRBY type offset increment] [OVERFLOW WRAP | SAT | FAIL]
BITFIELD_RO key [GET type offset] ...
```

Manipulate packed bits inside string values.

**Complexity:** O(1) for single bit access, O(N) for range operations

```bash
SETBIT bits 7 1
GETBIT bits 7
BITCOUNT bits
BITFIELD bits INCRBY u8 0 1 GET u8 0
```

## HyperLogLog

```
PFADD key element [element ...]
PFCOUNT key [key ...]
PFMERGE destkey sourcekey [sourcekey ...]
```

Approximate cardinality operations.

**Complexity:** O(1) per element (amortized)

```bash
PFADD uv:2026-03-05 user:1 user:2 user:3
PFCOUNT uv:2026-03-05
PFMERGE uv:week uv:2026-03-01 uv:2026-03-02
```

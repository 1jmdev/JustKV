# List Commands

Lists are ordered sequences of strings with fast push/pop at both ends.

## Push / Pop / Length

```
LPUSH key element [element ...]
RPUSH key element [element ...]
LPOP key [count]
RPOP key [count]
LLEN key
```

Add, remove, and count list elements.

**Complexity:** O(1) per edge push/pop, O(N) when using `count`

```bash
RPUSH queue "task1" "task2"
LPUSH queue "urgent"
LPOP queue
LLEN queue
```

## Indexing and Ranges

```
LINDEX key index
LSET key index element
LRANGE key start stop
LTRIM key start stop
LPOS key element [RANK rank] [COUNT num-matches] [MAXLEN len]
```

Read, overwrite, trim, and locate elements by position.

**Complexity:** O(N)

```bash
RPUSH list a b c d e
LINDEX list 2
LRANGE list 0 -1
LPOS list c
LTRIM list 0 2
```

## Insert and Move

```
LINSERT key BEFORE | AFTER pivot element
LMOVE source destination LEFT | RIGHT LEFT | RIGHT
BRPOPLPUSH source destination timeout
```

Insert around a pivot or atomically move values between lists.

**Complexity:** O(N) for `LINSERT`, O(1) for `LMOVE` / `BRPOPLPUSH`

```bash
LINSERT list BEFORE b "x"
LMOVE src dst RIGHT LEFT
BRPOPLPUSH pending processing 30
```

## Multi-Pop Commands

```
LMPOP numkeys key [key ...] LEFT | RIGHT [COUNT count]
BLMPOP timeout numkeys key [key ...] LEFT | RIGHT [COUNT count]
```

Pop one or many elements from the first non-empty list.

**Complexity:** O(N) where N is number of returned elements

```bash
LMPOP 2 queue:high queue:normal LEFT COUNT 5
BLMPOP 10 2 queue:high queue:normal RIGHT COUNT 1
```

## Blocking Pops

```
BLPOP key [key ...] timeout
BRPOP key [key ...] timeout
```

Block until an element is available or timeout expires.

**Complexity:** O(1)

```bash
BLPOP queue:jobs 30
BRPOP queue:jobs queue:retry 5
```

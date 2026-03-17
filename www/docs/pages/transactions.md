---
title: Transactions — BetterKV
description: Use MULTI/EXEC transactions in BetterKV for atomic command execution. Redis-compatible transactional workflows with WATCH for optimistic locking.
head:
  - - meta
    - property: og:title
      content: Transactions — BetterKV
  - - meta
    - property: og:description
      content: Use MULTI/EXEC transactions in BetterKV for atomic command execution. Redis-compatible transactional workflows with WATCH for optimistic locking.
  - - meta
    - property: og:url
      content: https://docs.betterkv.com/transactions
  - - meta
    - name: twitter:card
      content: summary_large_image
  - - meta
    - name: twitter:title
      content: Transactions — BetterKV
  - - meta
    - name: twitter:description
      content: Use MULTI/EXEC transactions in BetterKV for atomic command execution. Redis-compatible transactional workflows with WATCH for optimistic locking.
---

# Transactions

BetterKV supports Redis-style transactional workflows, but the real developer question is when to use transactions versus Lua scripts versus plain pipelining.

## MULTI and EXEC

```bash
MULTI
SET foo bar
INCR counter
LPUSH jobs a
EXEC
```

Commands queue after `MULTI` and execute together at `EXEC`.

## DISCARD

```bash
MULTI
SET foo oops
DISCARD
```

## WATCH for optimistic concurrency

```bash
WATCH balance:alice balance:bob
MULTI
SET balance:alice 900
SET balance:bob 1100
EXEC
```

If a watched key changes before `EXEC`, the transaction aborts and the client should retry.

## When to use what

| Need | Best fit |
| --- | --- |
| simple atomic batch | `MULTI` / `EXEC` |
| compare-and-set | `WATCH` + `MULTI` |
| conditional server-side logic | Lua |
| raw throughput with independent commands | pipelining |

## Error model

- syntax or queueing errors can abort the transaction before `EXEC`
- runtime errors inside `EXEC` do not roll back earlier successful commands

## Comparison guidance

If you benchmark BetterKV against Redis and Valkey, transactions are another place to compare tail latency rather than only throughput. Publish contention-heavy cases, not only no-conflict happy paths.

## Related docs

- [Lua Scripting](lua-scripting)
- [Commands Reference](commands/)

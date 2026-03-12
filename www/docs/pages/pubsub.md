# Pub/Sub

Pub/Sub is the simplest messaging surface in BetterKV, but it is not the best place to sell durability or replay. Use this page to explain where Pub/Sub fits, what is currently exposed in the command surface, and when Streams are the better choice.

## Where Pub/Sub fits

Use Pub/Sub when you need:

- low-overhead fan-out
- transient notifications
- best-effort delivery
- one producer to many listeners

Use Streams when you need:

- acknowledgements
- replay
- consumer groups
- durable event processing

## Basic flow

```bash
SUBSCRIBE chat:general
PUBLISH chat:general "hello"
UNSUBSCRIBE chat:general
```

Pattern subscriptions:

```bash
PSUBSCRIBE metrics:*
PUNSUBSCRIBE metrics:*
```

## Keyspace notifications

```ini title="betterkv.conf"
notify-keyspace-events "KEA"
```

```bash
SUBSCRIBE __keyevent@0__:expired
SUBSCRIBE __keyspace@0__:user:1
```

## Practical guidance

- dedicate a separate connection for subscribers
- do not treat Pub/Sub as a durable queue
- prefer Streams for background processing or at-least-once delivery

## Product positioning

If Pub/Sub commands are present in the command surface but not fully active in the current runtime, say that clearly and point developers to the per-command docs. The command reference already marks declared-but-unsupported commands.

## Better alternative for serious pipelines

If your users are comparing BetterKV with Redis and Valkey for event-driven systems, steer them toward Streams, where latency and replay behavior are easier to reason about in production.

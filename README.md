# valkey

`valkey` is a lightweight Redis-compatible in-memory server written in Rust.

## Features

- RESP2 protocol parsing and encoding.
- Multi-threaded Tokio TCP server.
- Sharded in-memory key/value store for concurrent access.
- Background key expiration sweeper.
- Redis-compatible command subset:
  - `PING`, `ECHO`
  - `GET`, `SET`, `DEL`, `EXISTS`
  - `INCR`, `MGET`, `MSET`
  - `EXPIRE`, `TTL`

## Run

```bash
cargo run -- --bind 127.0.0.1 --port 6379
```

Optional tuning:

- `--shards <N>`: shard count (defaults to CPU-based power-of-two).
- `--sweep-interval-ms <N>`: expiration sweep interval in milliseconds.

## Test

```bash
cargo test
```

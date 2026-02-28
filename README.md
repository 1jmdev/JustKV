# JustKV

`JustKV` is a lightweight Redis-compatible in-memory server written in Rust.

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

## Workspace

- `crates/justkv`: JustKV server library (protocol, engine, networking).
- `crates/justkv-cli`: CLI binary package exposing the `justkv` executable.
- `crates/justkv-server`: Redis-style server binary exposing the `justkv-server` executable.

## Run

```bash
cargo run -p justkv-server -- --bind 127.0.0.1 --port 6379

# redis-server style
cargo run -p justkv-server -- ./redis.conf --port 6379

# show compatible startup flags
cargo run -p justkv-server -- --help
```

Optional tuning:

- `--shards <N>`: shard count (defaults to CPU-based power-of-two).
- `--sweep-interval-ms <N>`: expiration sweep interval in milliseconds.

## Test

```bash
cargo test --workspace
```

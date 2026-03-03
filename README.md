# JustKV

`JustKV` is a lightweight Redis-compatible in-memory server written in Rust.

## Features

- RESP2 protocol parsing and encoding.
- Multi-threaded Tokio TCP server.
- Sharded in-memory key/value store for concurrent access.
- Background key expiration sweeper.
- Redis-compatible for common commands

## Workspace

- `crates/justkv`: JustKV server library (protocol, engine, networking).
- `crates/justkv-cli`: CLI binary package exposing the `justkv` executable.
- `crates/justkv-server`: Redis-style server binary exposing the `justkv-server` executable.
- `crates/benchmark-cli`: Redis-benchmark style binary exposing the `justkv-benchmark` executable.

## Run

```bash
cargo run -p justkv-server -- --bind 127.0.0.1 --port 6379

# redis-server style
cargo run -p justkv-server -- ./redis.conf --port 6379

# show compatible startup flags
cargo run -p justkv-server -- --help

# redis-benchmark style load test
cargo run -p justkv-benchmark -- -h 127.0.0.1 -p 6379 -c 100 -n 100000 -P 16
```

Optional tuning:

- `--io-threads <N>`: TCP accept/runtime worker threads.
- `--shards <N>`: shard count (defaults to CPU-based power-of-two).
- `--sweep-interval-ms <N>`: expiration sweep interval in milliseconds.

## Test

```bash
cargo test --workspace
```

## Latency Profiling

You can enable lightweight server-side latency profiling without changing any client code.

```bash
JUSTKV_PROFILE=1 \
JUSTKV_PROFILE_INTERVAL_SECS=2 \
JUSTKV_PROFILE_SLOW_MS=1 \
cargo run -p justkv-server --release -- --port 6379
```

Profiler output is written to stderr every interval and includes:

- Stage totals (`parse`, `execute`, `encode`, `write`) for the last window.
- Top commands by cumulative command execution time.
- Per-command count, avg/max execution time, and slow-command ratio.

Environment variables:

- `JUSTKV_PROFILE`: enable profiler (`1`, `true`, `yes`, `on`).
- `JUSTKV_PROFILE_INTERVAL_SECS`: report window length in seconds (default `5`).
- `JUSTKV_PROFILE_SLOW_MS`: command slow threshold in milliseconds (default `5`).
- `JUSTKV_PROFILE_LONG_MS`: end-to-end request slow threshold in milliseconds (defaults to `JUSTKV_PROFILE_SLOW_MS`).
- `JUSTKV_PROFILE_SLOW_SAMPLES`: number of slow request samples kept per report window (default `8`, max `64`).

Automated per-command profiling run:

```bash
bun run profile.ts
```

This script builds `justkv-server`, runs isolated workloads (`SET`, `GET`, `INCR`, `HSET`, `SADD`, `LPUSH`, `LRANGE`, `EXPIRE`) plus a mixed burst workload, then writes stage timings and long-request breakdowns to `profile-results.json`.

# Installation

Install BetterKV the same way you would install Redis or Valkey, then validate behavior with your existing clients and workloads.

## Packaging and license

- BetterKV is licensed under **Elastic License 2.0**.
- The main evaluation path should be identical to Redis and Valkey: start a server, point an existing client at port `6379`, then run your workload.
- If you are benchmarking, keep hardware, kernel settings, and persistence settings equivalent across BetterKV, Valkey, and Redis.

## Docker

```bash
docker pull betterkv/betterkv:latest

docker run -d   --name betterkv   -p 6379:6379   -v /local/data:/data   betterkv/betterkv:latest
```

## Docker Compose

```yaml title="docker-compose.yml"
services:
  betterkv:
    image: betterkv/betterkv:latest
    ports:
      - "6379:6379"
    volumes:
      - /local/data:/data
```

## Build from source

```bash
git clone https://github.com/1jmdev/BetterKV.git
cd BetterKV
cargo build --release
```

Run tests before local evaluation:

```bash
cargo test
```

Start the server:

```bash
./target/release/betterkv-server
```

## Verify the install

```bash
./target/release/betterkv-server --version
redis-cli -h 127.0.0.1 -p 6379 ping
# PONG
```

## Benchmarking guidance

If the reason you are installing BetterKV is comparison, keep the test honest:

- use the same client library for BetterKV, Redis, and Valkey
- keep persistence mode aligned across all three
- pin CPU and memory limits consistently
- compare tail latency, not only throughput
- include an expiry-heavy scenario and a scripting scenario

## What to publish later

This docs set assumes the public performance position will be:

- BetterKV is **up to 10x faster** than Redis and Valkey on published workloads
- **Valkey p50 is BetterKV p99.9** on selected benchmark paths

Replace these placeholders with your final benchmark tables before release.

## Next steps

- [Quick Start](quick-start)
- [Configuration](configuration)
- [Security](security)

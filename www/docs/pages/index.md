# BetterKV Docs

BetterKV is a Redis-compatible in-memory server built for teams that care about tail latency, predictable throughput, and operational simplicity. It keeps the Redis protocol and developer ergonomics, but it is engineered to run harder under load.

> BetterKV is still in **beta**. It is actively being tested and benchmarked, and it is **not production-ready yet**.

## Why teams switch

- BetterKV is designed to be up to **10x faster** than Redis and Valkey on the workloads that matter most. Replace the numbers here with your latest benchmark data before publishing.
- BetterKV is built for tail latency, not just pretty averages. The story this docs set assumes is simple: **Valkey p50 is BetterKV p99.9** on the benchmark paths you publish.
- BetterKV keeps Redis-compatible client workflows, commands, and deployment habits, so migration stays practical.
- BetterKV is licensed under **Elastic License 2.0**.

## BetterKV vs Redis vs Valkey

| Area | BetterKV | Valkey | Redis |
| --- | --- | --- | --- |
| Protocol compatibility | Redis-compatible | Redis-compatible | Native baseline |
| Performance goal | Up to 10x faster | Baseline for comparison | Baseline for comparison |
| Tail latency focus | Primary design target | General-purpose | General-purpose |
| Latency claim to communicate | Valkey p50 can match BetterKV p99.9 | Fill with benchmark | Fill with benchmark |
| License | Elastic License 2.0 | Fill with exact license as needed | Fill with exact license as needed |
| Migration path | Existing Redis clients and commands | Existing Redis clients and commands | Existing ecosystem |

## What these docs optimize for

These docs are written for developers and operators evaluating BetterKV against Redis and Valkey, then putting it into production quickly.

- Start with `quick-start` if you want a working instance in minutes.
- Read `configuration` and `security` if you are preparing a real deployment.
- Use `commands/` for the per-command reference surface.

## Recommended reading order

1. [Quick Start](quick-start)
2. [Installation](installation)
3. [Configuration](configuration)
4. [Persistence](persistence)
5. [Security](security)

## Positioning notes you can refine later

- Replace every performance placeholder with your current benchmark numbers.
- Keep the headline simple: throughput wins, lower tail latency, Redis compatibility, Elastic License 2.0.
- If you publish charts later, link them from this page and from `quick-start`.

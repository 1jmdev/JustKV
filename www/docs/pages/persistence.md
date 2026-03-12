# Persistence

Persistence is where many in-memory systems stop looking fast once the workload becomes real. BetterKV aims to keep stronger latency behavior than Redis and Valkey even when durability is enabled, but your benchmark methodology still needs to be explicit.

## Persistence modes

BetterKV supports the familiar Redis-style model:

- RDB snapshots for compact point-in-time recovery
- AOF for durable write replay
- both together for balanced recovery and durability

## RDB snapshots

```ini title="betterkv.conf"
save 3600 1
save 300 100
save 60 10000

dbfilename dump.rdb
dir /var/lib/betterkv
rdbcompression yes
rdbchecksum yes
```

RDB is the lowest-overhead option when you want smaller files and faster cold restarts.

## AOF

```ini title="betterkv.conf"
appendonly yes
appendfilename "appendonly.aof"
appendfsync everysec
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb
```

AOF is the better default when durability matters more than absolute write-path minimalism.

## Recommended production profile

```ini title="betterkv.conf"
save 3600 1
save 300 100
save 60 10000
appendonly yes
appendfsync everysec
```

## Benchmarking persistence correctly

If you claim BetterKV is up to 10x faster than Redis and Valkey, publish persistence context next to the numbers.

At minimum, state:

- whether persistence was disabled, RDB-only, AOF-only, or both
- the `appendfsync` mode
- whether background rewrite/save activity was active
- the dataset size and write amplification
- p50, p95, p99, and p99.9 under the same persistence profile for all systems

## Operational advice

- Use RDB for backup portability and fast recovery points.
- Use AOF when you need tighter durability.
- Use both when you want a practical production default.
- Put persistence files on fast storage if you care about long-tail stability.

## Manual operations

```bash
betterkv-cli BGSAVE
betterkv-cli BGREWRITEAOF
betterkv-cli LASTSAVE
```

## Positioning guidance

Persistence is one of the best places to show BetterKV's value. If your benchmark story says "Valkey p50 is BetterKV p99.9," include a persistence-enabled workload so the claim reflects production reality rather than memory-only microbenchmarks.

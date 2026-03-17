---
title: Configuration — BetterKV
description: Configure BetterKV for your workload. Redis-compatible config model with tuning options for latency, threading, persistence, TLS, and authentication.
head:
  - - meta
    - property: og:title
      content: Configuration — BetterKV
  - - meta
    - property: og:description
      content: Configure BetterKV for your workload. Redis-compatible config model with tuning options for latency, threading, persistence, TLS, and authentication.
  - - meta
    - property: og:url
      content: https://docs.betterkv.com/configuration
  - - meta
    - name: twitter:card
      content: summary_large_image
  - - meta
    - name: twitter:title
      content: Configuration — BetterKV
  - - meta
    - name: twitter:description
      content: Configure BetterKV for your workload. Redis-compatible config model with tuning options for latency, threading, persistence, TLS, and authentication.
---

# Configuration

BetterKV keeps the Redis-style operational model, but the goal is different: tighter latency distributions, less surprise under pressure, and simpler performance tuning.

## Configuration model

Start BetterKV with a config file:

```bash
betterkv-server /etc/betterkv/betterkv.conf
```

Override individual settings on the command line:

```bash
betterkv-server /etc/betterkv/betterkv.conf --port 6380 --loglevel verbose
```

## Recommended baseline

Use this as a practical starting point for a single primary in production:

```ini title="betterkv.conf"
bind 127.0.0.1 10.0.0.5
port 6379
protected-mode yes
maxclients 10000

dir /var/lib/betterkv
dbfilename dump.rdb

appendonly yes
appendfsync everysec

save 900 1
save 300 10
save 60 10000
```

## Performance-sensitive settings

### Network

```ini title="betterkv.conf"
tcp-keepalive 300
# unixsocket /tmp/betterkv.sock
# unixsocketperm 700
```

Use a Unix socket for the lowest local-call overhead when the application and BetterKV share a host.

### Memory and eviction

```ini title="betterkv.conf"
maxmemory 4gb
maxmemory-policy allkeys-lru
maxmemory-samples 10
```

Choose eviction deliberately. If you are comparing with Redis or Valkey, make sure eviction policy and dataset pressure are identical.

### Persistence

```ini title="betterkv.conf"
appendonly yes
appendfsync everysec
save 3600 1
save 300 100
save 60 10000
```

Persistence settings can materially change latency. For fair benchmarks, align them across BetterKV, Valkey, and Redis.

### Replication

```ini title="betterkv.conf"
replicaof 192.168.1.10 6379
masterauth your_primary_password
replica-read-only yes
```

### ACL and auth

```ini title="betterkv.conf"
requirepass your_strong_password_here
aclfile /etc/betterkv/users.acl
```

## Tuning for comparisons

If your benchmark claims are going to say BetterKV is up to 10x faster, make the setup defensible:

- pin the same CPU count for each server
- use equivalent persistence settings
- keep the same client pipeline depth
- report p50, p95, p99, and p99.9
- call out whether the workload is read-heavy, write-heavy, expiry-heavy, or script-heavy

## Runtime changes

```bash
betterkv-cli CONFIG GET maxmemory
betterkv-cli CONFIG SET maxmemory 8gb
betterkv-cli CONFIG REWRITE
```

## Operator note

The marketing line belongs in benchmarks, not in config defaults. The configuration story should support the benchmark story: stable latency, predictable behavior, and fewer long-tail spikes than Redis and Valkey.

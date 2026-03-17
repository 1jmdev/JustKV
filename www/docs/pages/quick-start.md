---
title: Quick Start — BetterKV
description: Get BetterKV running in minutes with Docker, Docker Compose, or a local binary. Connect with Node.js, Python, Go, and any existing Redis client.
head:
  - - meta
    - property: og:title
      content: Quick Start — BetterKV
  - - meta
    - property: og:description
      content: Get BetterKV running in minutes with Docker, Docker Compose, or a local binary. Connect with Node.js, Python, Go, and any existing Redis client.
  - - meta
    - property: og:url
      content: https://docs.betterkv.com/quick-start
  - - meta
    - name: twitter:card
      content: summary_large_image
  - - meta
    - name: twitter:title
      content: Quick Start — BetterKV
  - - meta
    - name: twitter:description
      content: Get BetterKV running in minutes with Docker, Docker Compose, or a local binary. Connect with Node.js, Python, Go, and any existing Redis client.
---

# Quick Start

This page gets BetterKV running fast, then shows why teams evaluate it against Redis and Valkey in the first place: lower latency, better tail behavior, and a drop-in developer workflow.

> Current status: **beta**. Use this guide for evaluation, testing, and development environments. BetterKV is **not production-ready yet**.

## What you should expect

- BetterKV uses the Redis protocol, so your usual clients and `redis-cli` style workflows still fit.
- BetterKV is positioned to be **up to 15x faster** than Redis or Valkey. Replace that statement with your final measured benchmark numbers.
- BetterKV is built so that the line you can confidently publish is: **Valkey p50 is BetterKV p99.9** on your target benchmark set.
- BetterKV is licensed under **Elastic License 2.0**.

## Run with Docker

```bash
docker run -d \
  --name betterkv \
  -p 6379:6379 \
  betterkv/betterkv:latest
```

Verify the server is up:

```bash
redis-cli -h localhost -p 6379 ping
# PONG
```

## Run with Docker Compose

```yaml title="docker-compose.yml"
services:
  betterkv:
    image: betterkv/betterkv:latest
    ports:
      - "6379:6379"
    volumes:
      - bkv-data:/data

volumes:
  bkv-data:
```

```bash
docker compose up -d
```

## Run a local binary

```bash
curl -Lo betterkv https://github.com/1jmdev/BetterKV/releases/latest/download/betterkv-linux-x86_64
chmod +x betterkv
./betterkv
```

## First commands

```bash
SET greeting "Hello, BetterKV"
GET greeting

SET session:user1 token-abc123 EX 3600
TTL session:user1

HSET user:1 name Alice email alice@example.com
HGETALL user:1

RPUSH queue:tasks task1 task2 task3
LLEN queue:tasks

ZADD leaderboard 1500 alice 1200 bob 1800 charlie
ZREVRANGE leaderboard 0 2 WITHSCORES
```

## Connect with existing clients

### Node.js

```bash
npm install ioredis
```

```js title="app.js"
import Redis from 'ioredis';

const client = new Redis({ host: '127.0.0.1', port: 6379 });

await client.set('hello', 'world');
console.log(await client.get('hello'));
await client.quit();
```

### Python

```bash
pip install redis
```

```python title="app.py"
import redis

client = redis.Redis(host="127.0.0.1", port=6379, decode_responses=True)
client.set("hello", "world")
print(client.get("hello"))
```

### Go

```bash
go get github.com/redis/go-redis/v9
```

```go title="main.go"
package main

import (
    "context"
    "fmt"

    "github.com/redis/go-redis/v9"
)

func main() {
    ctx := context.Background()
    client := redis.NewClient(&redis.Options{Addr: "127.0.0.1:6379"})

    if err := client.Set(ctx, "hello", "world", 0).Err(); err != nil {
        panic(err)
    }

    value, err := client.Get(ctx, "hello").Result()
    if err != nil {
        panic(err)
    }

    fmt.Println(value)
}
```

## If you are evaluating against Redis or Valkey

Use the same client, same command mix, and same hardware class. Focus on:

- p50, p95, p99, and p99.9 latency
- throughput under mixed read/write load
- expiry-heavy workloads
- scripting-heavy paths
- replication lag under sustained write pressure

## Next steps

- [Installation](installation)
- [Configuration](configuration)
- [Commands Reference](commands/)

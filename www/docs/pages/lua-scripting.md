# Lua Scripting

Lua is one of the best places to explain why BetterKV exists. Server-side logic amplifies latency problems in Redis-compatible systems, so better tail behavior matters even more here.

## Positioning

- BetterKV exposes Redis-style Lua scripting primitives.
- This page should support a benchmark story where BetterKV is significantly faster than Redis and Valkey on scripting-heavy paths.
- If your measurements support it, call out that **Valkey p50 is BetterKV p99.9** for the scripting benchmark you publish.

## Execute a script

```bash
EVAL "return redis.call('GET', KEYS[1])" 1 profile:1
```

## Cache and reuse a script

```bash
SCRIPT LOAD "return redis.call('GET', KEYS[1])"
EVALSHA <sha1> 1 profile:1
```

## Useful patterns

### Conditional update

```bash
EVAL "
  local value = redis.call('GET', KEYS[1])
  if value == ARGV[1] then
    return redis.call('SET', KEYS[1], ARGV[2])
  end
  return 0
" 1 mykey old new
```

### Rate limiting

```lua
local key = KEYS[1]
local limit = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local now = tonumber(ARGV[3])

redis.call('ZREMRANGEBYSCORE', key, 0, now - window)
local count = redis.call('ZCARD', key)
if count < limit then
  redis.call('ZADD', key, now, now)
  redis.call('EXPIRE', key, window)
  return 1
end
return 0
```

### Safe lock release

```lua
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return redis.call('DEL', KEYS[1])
end
return 0
```

## Benchmark advice

If scripting is part of your public comparison versus Redis and Valkey, report:

- script size and command mix
- hot-cache vs cold-cache behavior
- `EVAL` vs `EVALSHA`
- contention impact on unrelated commands
- p50, p99, and p99.9

## Developer guidance

- Prefer `EVALSHA` after the first load.
- Keep scripts focused and short.
- Use `KEYS` only for key names and `ARGV` for values.
- Avoid expensive keyspace-wide operations inside scripts.

## Command reference

See `commands/eval`, `commands/evalsha`, `commands/eval-ro`, `commands/evalsha-ro`, and `commands/script` for command-level details.

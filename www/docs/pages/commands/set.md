# Set Commands

Sets are unordered collections of unique strings.

## Core Set Commands

```
SADD key member [member ...]
SREM key member [member ...]
SISMEMBER key member
SMEMBERS key
SCARD key
SPOP key [count]
SRANDMEMBER key [count]
SMOVE source destination member
SSCAN key cursor [MATCH pattern] [COUNT count]
```

Add/remove members, test membership, and iterate sets.

**Complexity:** O(1) per member for basic ops, O(N) for full scans and bulk returns

```bash
SADD tags redis cache database
SISMEMBER tags redis
SMEMBERS tags
SPOP tags
SSCAN tags 0 MATCH "re*" COUNT 100
```

## Set Operations

```ts
SINTER key [key ...]
SINTERSTORE destination key [key ...]
SINTERCARD numkeys key [key ...] [LIMIT limit]
SUNION key [key ...]
SUNIONSTORE destination key [key ...]
SDIFF key [key ...]
SDIFFSTORE destination key [key ...]
```

Compute intersections, unions, and differences in-memory or store them.

**Complexity:** O(N) to O(N*M) depending on command and key sizes

```bash
SINTER team:a team:b
SINTERCARD 2 team:a team:b
SUNIONSTORE team:all team:a team:b team:c
SDIFFSTORE team:only-a team:a team:b
```

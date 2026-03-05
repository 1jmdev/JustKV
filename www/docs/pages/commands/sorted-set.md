# Sorted Set Commands

Sorted sets map members to scores and keep members ordered by score.

## Write and Update

```
ZADD key [NX | XX] [GT | LT] [CH] [INCR] score member [score member ...]
ZREM key member [member ...]
ZINCRBY key increment member
```

Add, remove, and update scored members.

**Complexity:** O(log N) per member

```bash
ZADD leaderboard 1200 bob 1800 alice
ZINCRBY leaderboard 50 bob
ZREM leaderboard alice
```

## Read by Rank / Score

```
ZRANGE key start stop [BYSCORE] [REV] [LIMIT offset count] [WITHSCORES]
ZREVRANGE key start stop [WITHSCORES]
ZRANGEBYSCORE key min max [WITHSCORES] [LIMIT offset count]
ZREVRANGEBYSCORE key max min [WITHSCORES] [LIMIT offset count]
ZRANK key member
ZREVRANK key member
ZSCORE key member
ZMSCORE key member [member ...]
ZRANDMEMBER key [count [WITHSCORES]]
```

Query sorted sets by rank, score range, or random selection.

**Complexity:** O(log N + M) for range queries

```bash
ZRANGE leaderboard 0 -1 WITHSCORES
ZRANGEBYSCORE leaderboard 1000 2000 WITHSCORES
ZREVRANK leaderboard bob
ZMSCORE leaderboard bob carol
```

## Count / Pop / Remove Ranges

```
ZCARD key
ZCOUNT key min max
ZPOPMIN key [count]
ZPOPMAX key [count]
BZPOPMIN key [key ...] timeout
BZPOPMAX key [key ...] timeout
ZREMRANGEBYRANK key start stop
```

Count members, pop min/max members, or remove rank ranges.

**Complexity:** O(1) for `ZCARD`, O(log N) for `ZCOUNT`, O(log N * M) for pops/removals

```bash
ZCOUNT leaderboard 1000 2000
ZPOPMIN jobs 1
BZPOPMAX leaderboard 10
ZREMRANGEBYRANK leaderboard 0 9
```

## Set Algebra and Iteration

```
ZINTER numkeys key [key ...] [WEIGHTS weight ...] [AGGREGATE SUM | MIN | MAX]
ZUNION numkeys key [key ...] [WEIGHTS weight ...] [AGGREGATE SUM | MIN | MAX]
ZDIFF numkeys key [key ...] [WITHSCORES]
ZMPOP numkeys key [key ...] MIN | MAX [COUNT count]
BZMPOP timeout numkeys key [key ...] MIN | MAX [COUNT count]
ZSCAN key cursor [MATCH pattern] [COUNT count]
```

Compute unions/intersections/differences, pop from multiple keys, and iterate incrementally.

**Complexity:** Depends on input cardinality; `ZSCAN` is O(1) per call, O(N) total

```bash
ZINTER 2 lb:week lb:month
ZUNION 2 lb:week lb:alltime WEIGHTS 2 1
ZDIFF 2 lb:all lb:banned WITHSCORES
ZMPOP 2 queue:a queue:b MIN COUNT 5
```

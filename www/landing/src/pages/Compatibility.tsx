import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
import { CheckIcon } from "lucide-react";
import { Seo } from "@/components/Seo";

const commandGroups = [
    {
        name: "Connection",
        commands: [
            "AUTH",
            "CLIENT",
            "ECHO",
            "HELLO",
            "PING",
            "QUIT",
            "SELECT",
        ],
    },
    {
        name: "Server",
        commands: [
            "COMMAND",
            "CONFIG",
            "DBSIZE",
            "FLUSHALL",
            "FLUSHDB",
        ],
    },
    {
        name: "Keyspace",
        commands: [
            "COPY",
            "DEL",
            "DUMP",
            "EXISTS",
            "EXPIRE",
            "EXPIREAT",
            "KEYS",
            "MOVE",
            "OBJECT",
            "PERSIST",
            "PEXPIRE",
            "PEXPIREAT",
            "PTTL",
            "RANDOMKEY",
            "RENAME",
            "RENAMENX",
            "RESTORE",
            "SCAN",
            "SORT",
            "TOUCH",
            "TTL",
            "TYPE",
            "UNLINK",
        ],
    },
    {
        name: "String",
        commands: [
            "APPEND",
            "BITCOUNT",
            "BITFIELD",
            "BITFIELD_RO",
            "BITOP",
            "BITPOS",
            "DECR",
            "DECRBY",
            "DELEX",
            "DIGEST",
            "GET",
            "GETBIT",
            "GETDEL",
            "GETEX",
            "GETRANGE",
            "GETSET",
            "INCR",
            "INCRBY",
            "INCRBYFLOAT",
            "LCS",
            "MGET",
            "MSET",
            "MSETEX",
            "MSETNX",
            "PFADD",
            "PFCOUNT",
            "PFMERGE",
            "PSETEX",
            "SET",
            "SETBIT",
            "SETEX",
            "SETNX",
            "SETRANGE",
            "STRLEN",
            "SUBSTR",
        ],
    },
    {
        name: "Hash",
        commands: [
            "HDEL",
            "HEXISTS",
            "HGET",
            "HGETALL",
            "HINCRBY",
            "HINCRBYFLOAT",
            "HKEYS",
            "HLEN",
            "HMGET",
            "HMSET",
            "HRANDFIELD",
            "HSCAN",
            "HSET",
            "HSETNX",
            "HSTRLEN",
            "HVALS",
        ],
    },
    {
        name: "List",
        commands: [
            "BLMPOP",
            "BLPOP",
            "BRPOP",
            "BRPOPLPUSH",
            "LINDEX",
            "LINSERT",
            "LLEN",
            "LMOVE",
            "LMPOP",
            "LPOP",
            "LPOS",
            "LPUSH",
            "LPUSHX",
            "LRANGE",
            "LREM",
            "LSET",
            "LTRIM",
            "RPOP",
            "RPOPLPUSH",
            "RPUSH",
            "RPUSHX",
        ],
    },
    {
        name: "Set",
        commands: [
            "SADD",
            "SCARD",
            "SDIFF",
            "SDIFFSTORE",
            "SINTER",
            "SINTERCARD",
            "SINTERSTORE",
            "SISMEMBER",
            "SMEMBERS",
            "SMISMEMBER",
            "SMOVE",
            "SPOP",
            "SRANDMEMBER",
            "SREM",
            "SSCAN",
            "SUNION",
            "SUNIONSTORE",
        ],
    },
    {
        name: "Sorted Set",
        commands: [
            "BZMPOP",
            "BZPOPMAX",
            "BZPOPMIN",
            "ZADD",
            "ZCARD",
            "ZCOUNT",
            "ZDIFF",
            "ZDIFFSTORE",
            "ZINCRBY",
            "ZINTER",
            "ZINTERSTORE",
            "ZLEXCOUNT",
            "ZMPOP",
            "ZMSCORE",
            "ZPOPMAX",
            "ZPOPMIN",
            "ZRANDMEMBER",
            "ZRANGE",
            "ZRANGEBYLEX",
            "ZRANGEBYSCORE",
            "ZRANGESTORE",
            "ZRANK",
            "ZREM",
            "ZREMRANGEBYLEX",
            "ZREMRANGEBYRANK",
            "ZREMRANGEBYSCORE",
            "ZREVRANGE",
            "ZREVRANGEBYLEX",
            "ZREVRANGEBYSCORE",
            "ZREVRANK",
            "ZSCAN",
            "ZSCORE",
            "ZUNION",
            "ZUNIONSTORE",
        ],
    },
    {
        name: "Geo",
        commands: [
            "GEOADD",
            "GEODIST",
            "GEOHASH",
            "GEOPOS",
            "GEORADIUS",
            "GEORADIUSBYMEMBER",
            "GEORADIUSBYMEMBER_RO",
            "GEORADIUS_RO",
            "GEOSEARCH",
            "GEOSEARCHSTORE",
        ],
    },
    {
        name: "Stream",
        commands: [
            "XACK",
            "XADD",
            "XAUTOCLAIM",
            "XCLAIM",
            "XDEL",
            "XDELEX",
            "XGROUP",
            "XLEN",
            "XPENDING",
            "XRANGE",
            "XREAD",
            "XREADGROUP",
            "XREVRANGE",
            "XTRIM",
        ],
    },
    {
        name: "Scripting",
        commands: [
            "EVAL",
            "EVALSHA",
            "EVALSHA_RO",
            "EVAL_RO",
            "SCRIPT",
        ],
    },
    {
        name: "Transaction",
        commands: [
            "DISCARD",
            "EXEC",
            "MULTI",
            "UNWATCH",
            "WATCH",
        ],
    },
    {
        name: "Pub/Sub",
        commands: [
            "PSUBSCRIBE",
            "PUBLISH",
            "PUBSUB",
            "PUNSUBSCRIBE",
            "SPUBLISH",
            "SSUBSCRIBE",
            "SUBSCRIBE",
            "SUNSUBSCRIBE",
            "UNSUBSCRIBE",
        ],
    },
    {
        name: "JSON",
        commands: [
            "JSON.ARRAPPEND",
            "JSON.ARRINDEX",
            "JSON.ARRINSERT",
            "JSON.ARRLEN",
            "JSON.ARRPOP",
            "JSON.ARRTRIM",
            "JSON.CLEAR",
            "JSON.DEBUG",
            "JSON.DEL",
            "JSON.FORGET",
            "JSON.GET",
            "JSON.MERGE",
            "JSON.MGET",
            "JSON.MSET",
            "JSON.NUMINCRBY",
            "JSON.NUMMULTBY",
            "JSON.OBJKEYS",
            "JSON.OBJLEN",
            "JSON.RESP",
            "JSON.SET",
            "JSON.STRAPPEND",
            "JSON.STRLEN",
            "JSON.TOGGLE",
            "JSON.TYPE",
        ],
    },
];

const clients = [
    { name: "redis-py", language: "Python", status: "Fully Compatible" },
    { name: "ioredis", language: "Node.js", status: "Fully Compatible" },
    { name: "node-redis", language: "Node.js", status: "Fully Compatible" },
    { name: "go-redis", language: "Go", status: "Fully Compatible" },
    { name: "Jedis", language: "Java", status: "Fully Compatible" },
    { name: "Lettuce", language: "Java", status: "Fully Compatible" },
    { name: "redis-rs", language: "Rust", status: "Fully Compatible" },
    { name: "StackExchange.Redis", language: "C#", status: "Fully Compatible" },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function CompatibilityPage() {
    return (
        <div>
            <Seo
                title="Redis Command Compatibility — BetterKV"
                description="BetterKV supports 225+ Redis commands across strings, hashes, lists, sets, sorted sets, streams, JSON, Lua scripting, pub/sub, and transactions. Drop-in Redis replacement."
                path="/compatibility"
            />
            <PageHeader
                badge="Compatibility"
                title="Redis command compatibility."
                description="This page follows the command reference in the docs so the landing page and documentation stay aligned."
            />

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Supported Commands
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Core Redis command groups and their support status.
                        </p>
                    </motion.div>

                    <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                        {commandGroups.map((group) => (
                            <motion.div
                                key={group.name}
                                {...fadeUp}
                                className="rounded-xl border border-border/50 bg-card p-5"
                            >
                                <h3 className="mb-3 text-sm font-semibold">
                                    {group.name}
                                </h3>
                                <ul className="space-y-1.5">
                                    {group.commands.map((cmd) => (
                                        <li
                                            key={cmd}
                                            className="flex items-center gap-2 text-sm"
                                        >
                                            <CheckIcon className="size-3.5 shrink-0 text-emerald-400" />
                                            <span className="text-foreground">
                                                {cmd}
                                            </span>
                                        </li>
                                    ))}
                                </ul>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Client Libraries
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Every major Redis client works out of the box.
                        </p>
                    </motion.div>

                    <motion.div
                        {...fadeUp}
                        className="mt-10 rounded-xl border border-border/50 overflow-x-auto"
                    >
                        <table className="w-full min-w-100 text-sm">
                            <thead>
                                <tr className="border-b border-border/50 bg-card">
                                    <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-6">
                                        Client
                                    </th>
                                    <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-6">
                                        Language
                                    </th>
                                    <th className="px-4 py-3.5 text-right font-medium text-muted-foreground sm:px-6">
                                        Status
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {clients.map((client, i) => (
                                    <tr
                                        key={client.name}
                                        className={
                                            i % 2 === 0 ? "bg-card/50" : ""
                                        }
                                    >
                                        <td className="px-4 py-3.5 font-mono text-sm sm:px-6">
                                            {client.name}
                                        </td>
                                        <td className="px-4 py-3.5 text-muted-foreground sm:px-6">
                                            {client.language}
                                        </td>
                                        <td className="px-4 py-3.5 text-right sm:px-6">
                                            <Badge variant="secondary">
                                                {client.status}
                                            </Badge>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </motion.div>
                </div>
            </section>
        </div>
    );
}

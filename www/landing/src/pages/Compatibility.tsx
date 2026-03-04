import { motion } from "framer-motion"
import { PageHeader } from "@/components/layout/PageHeader"
import { Badge } from "@/components/ui/badge"
import { CheckIcon, MinusIcon } from "lucide-react"

const commandGroups = [
  {
    name: "Strings",
    commands: [
      { name: "GET", supported: true },
      { name: "SET", supported: true },
      { name: "MGET", supported: true },
      { name: "MSET", supported: true },
      { name: "INCR / DECR", supported: true },
      { name: "APPEND", supported: true },
      { name: "GETRANGE", supported: true },
      { name: "SETEX / PSETEX", supported: true },
      { name: "SETNX", supported: true },
    ],
  },
  {
    name: "Hashes",
    commands: [
      { name: "HGET", supported: true },
      { name: "HSET", supported: true },
      { name: "HMGET / HMSET", supported: true },
      { name: "HDEL", supported: true },
      { name: "HGETALL", supported: true },
      { name: "HINCRBY", supported: true },
      { name: "HKEYS / HVALS", supported: true },
      { name: "HEXISTS", supported: true },
      { name: "HLEN", supported: true },
    ],
  },
  {
    name: "Lists",
    commands: [
      { name: "LPUSH / RPUSH", supported: true },
      { name: "LPOP / RPOP", supported: true },
      { name: "LRANGE", supported: true },
      { name: "LLEN", supported: true },
      { name: "LINDEX", supported: true },
      { name: "LSET", supported: true },
      { name: "LREM", supported: true },
      { name: "BLPOP / BRPOP", supported: true },
    ],
  },
  {
    name: "Sets",
    commands: [
      { name: "SADD", supported: true },
      { name: "SREM", supported: true },
      { name: "SMEMBERS", supported: true },
      { name: "SISMEMBER", supported: true },
      { name: "SCARD", supported: true },
      { name: "SUNION / SINTER", supported: true },
      { name: "SDIFF", supported: true },
      { name: "SPOP", supported: true },
    ],
  },
  {
    name: "Sorted Sets",
    commands: [
      { name: "ZADD", supported: true },
      { name: "ZREM", supported: true },
      { name: "ZRANGE", supported: true },
      { name: "ZRANGEBYSCORE", supported: true },
      { name: "ZRANK", supported: true },
      { name: "ZSCORE", supported: true },
      { name: "ZCARD", supported: true },
      { name: "ZINCRBY", supported: true },
    ],
  },
  {
    name: "Keys & Server",
    commands: [
      { name: "DEL", supported: true },
      { name: "EXISTS", supported: true },
      { name: "EXPIRE / TTL", supported: true },
      { name: "KEYS / SCAN", supported: true },
      { name: "TYPE", supported: true },
      { name: "PING", supported: true },
      { name: "INFO", supported: true },
      { name: "DBSIZE", supported: true },
      { name: "FLUSHDB", supported: true },
    ],
  },
  {
    name: "Pub/Sub",
    commands: [
      { name: "PUBLISH", supported: true },
      { name: "SUBSCRIBE", supported: true },
      { name: "UNSUBSCRIBE", supported: true },
      { name: "PSUBSCRIBE", supported: true },
    ],
  },
  {
    name: "Transactions",
    commands: [
      { name: "MULTI", supported: true },
      { name: "EXEC", supported: true },
      { name: "DISCARD", supported: true },
      { name: "WATCH", supported: false },
    ],
  },
]

const clients = [
  { name: "redis-py", language: "Python", status: "Fully Compatible" },
  { name: "ioredis", language: "Node.js", status: "Fully Compatible" },
  { name: "node-redis", language: "Node.js", status: "Fully Compatible" },
  { name: "go-redis", language: "Go", status: "Fully Compatible" },
  { name: "Jedis", language: "Java", status: "Fully Compatible" },
  { name: "Lettuce", language: "Java", status: "Fully Compatible" },
  { name: "redis-rs", language: "Rust", status: "Fully Compatible" },
  { name: "StackExchange.Redis", language: "C#", status: "Fully Compatible" },
]

const fadeUp = {
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, margin: "-100px" },
  transition: { duration: 0.5 },
}

export function CompatibilityPage() {
  return (
    <div>
      <PageHeader
        badge="Compatibility"
        title="100% Redis protocol compatible."
        description="Use your existing Redis clients, tools, and workflows. BetterKV speaks the same language."
      />

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <h2 className="text-2xl font-bold tracking-tight">Supported Commands</h2>
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
                <h3 className="mb-3 text-sm font-semibold">{group.name}</h3>
                <ul className="space-y-1.5">
                  {group.commands.map((cmd) => (
                    <li key={cmd.name} className="flex items-center gap-2 text-sm">
                      {cmd.supported ? (
                        <CheckIcon className="size-3.5 text-emerald-400" />
                      ) : (
                        <MinusIcon className="size-3.5 text-muted-foreground" />
                      )}
                      <span className={cmd.supported ? "text-foreground" : "text-muted-foreground"}>
                        {cmd.name}
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
            <h2 className="text-2xl font-bold tracking-tight">Client Libraries</h2>
            <p className="mt-2 text-muted-foreground">
              Every major Redis client works out of the box.
            </p>
          </motion.div>

          <motion.div {...fadeUp} className="mt-10 rounded-xl border border-border/50 overflow-x-auto">
            <table className="w-full min-w-[400px] text-sm">
              <thead>
                <tr className="border-b border-border/50 bg-card">
                  <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-6">Client</th>
                  <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-6">Language</th>
                  <th className="px-4 py-3.5 text-right font-medium text-muted-foreground sm:px-6">Status</th>
                </tr>
              </thead>
              <tbody>
                {clients.map((client, i) => (
                  <tr key={client.name} className={i % 2 === 0 ? "bg-card/50" : ""}>
                    <td className="px-4 py-3.5 font-mono text-sm sm:px-6">{client.name}</td>
                    <td className="px-4 py-3.5 text-muted-foreground sm:px-6">{client.language}</td>
                    <td className="px-4 py-3.5 text-right sm:px-6">
                      <Badge variant="secondary">{client.status}</Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </motion.div>
        </div>
      </section>

      <section className="py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <h2 className="text-2xl font-bold tracking-tight">Migration Guide</h2>
            <p className="mt-2 text-muted-foreground">
              Switching from Redis takes less than a minute.
            </p>
          </motion.div>

          <motion.div {...fadeUp} className="mt-10 space-y-6">
            <div className="rounded-xl border border-border/50 bg-card p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">1</div>
                <h3 className="text-sm font-semibold">Install BetterKV</h3>
              </div>
              <div className="rounded-lg bg-muted/50 p-4 font-mono text-sm">
                curl -fsSL https://betterkv.com/install.sh | sh
              </div>
            </div>

            <div className="rounded-xl border border-border/50 bg-card p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">2</div>
                <h3 className="text-sm font-semibold">Start the server</h3>
              </div>
              <div className="rounded-lg bg-muted/50 p-4 font-mono text-sm">
                betterkv-server --port 6380
              </div>
            </div>

            <div className="rounded-xl border border-border/50 bg-card p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">3</div>
                <h3 className="text-sm font-semibold">Update your connection string</h3>
              </div>
              <div className="rounded-lg bg-muted/50 p-4 font-mono text-sm">
                <div className="text-muted-foreground line-through">REDIS_URL=redis://localhost:6379</div>
                <div className="mt-1 text-primary">REDIS_URL=redis://localhost:6380</div>
              </div>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  )
}

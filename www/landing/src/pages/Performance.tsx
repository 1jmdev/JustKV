import { motion } from "framer-motion"
import { PageHeader } from "@/components/layout/PageHeader"
import { Badge } from "@/components/ui/badge"

const benchmarks = [
  { label: "GET (single key)", betterkv: "2,400,000", redis: "200,000", ratio: "12x" },
  { label: "SET (single key)", betterkv: "2,100,000", redis: "190,000", ratio: "11x" },
  { label: "HSET (hash)", betterkv: "1,800,000", redis: "170,000", ratio: "10.5x" },
  { label: "LPUSH (list)", betterkv: "1,950,000", redis: "180,000", ratio: "10.8x" },
  { label: "SADD (set)", betterkv: "2,000,000", redis: "175,000", ratio: "11.4x" },
  { label: "Pipeline (100 cmds)", betterkv: "15,000,000", redis: "1,200,000", ratio: "12.5x" },
]

const latencyData = [
  { label: "p50 latency", betterkv: "5 \u03BCs", redis: "80 \u03BCs" },
  { label: "p99 latency", betterkv: "15 \u03BCs", redis: "250 \u03BCs" },
  { label: "p99.9 latency", betterkv: "30 \u03BCs", redis: "800 \u03BCs" },
]

const architectureFeatures = [
  {
    title: "True Multi-threading",
    description: "Unlike Redis's single-threaded event loop, BetterKV uses a thread-per-core architecture. Each core handles its own shard of data independently, eliminating contention and lock overhead.",
  },
  {
    title: "Zero-copy Networking",
    description: "io_uring-based I/O with zero-copy buffer management. Data moves from the network card to your application without unnecessary memory copies.",
  },
  {
    title: "Lock-free Data Structures",
    description: "Core data structures use atomic operations and lock-free algorithms. No mutex contention, no priority inversion, no lock convoys.",
  },
  {
    title: "Optimized Memory Allocator",
    description: "Custom slab allocator designed for key-value workloads. Minimizes fragmentation and reduces allocation overhead to near-zero.",
  },
  {
    title: "RESP3 Protocol",
    description: "Full support for the RESP3 protocol with optimized parsing. Inline commands are parsed with SIMD acceleration where available.",
  },
  {
    title: "Adaptive Batching",
    description: "Automatically batches responses when the client can't keep up, maximizing throughput without sacrificing latency under normal load.",
  },
]

const fadeUp = {
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, margin: "-100px" },
  transition: { duration: 0.5 },
}

export function PerformancePage() {
  return (
    <div>
      <PageHeader
        badge="Performance"
        title="5-15x faster than Redis."
        description="Every layer is engineered for maximum throughput and minimum latency. Here's the data."
      />

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <h2 className="text-2xl font-bold tracking-tight">Throughput Benchmarks</h2>
            <p className="mt-2 text-muted-foreground">
              Operations per second, single node, 64 byte values, 50 concurrent connections.
            </p>
          </motion.div>

          <motion.div {...fadeUp} className="mt-10 rounded-xl border border-border/50 overflow-x-auto">
            <table className="w-full min-w-[520px] text-sm">
              <thead>
                <tr className="border-b border-border/50 bg-card">
                  <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-6">Operation</th>
                  <th className="px-4 py-3.5 text-right font-medium text-primary sm:px-6">BetterKV</th>
                  <th className="px-4 py-3.5 text-right font-medium text-muted-foreground sm:px-6">Redis</th>
                  <th className="px-4 py-3.5 text-right font-medium text-muted-foreground sm:px-6">Improvement</th>
                </tr>
              </thead>
              <tbody>
                {benchmarks.map((row, i) => (
                  <tr key={row.label} className={i % 2 === 0 ? "bg-card/50" : ""}>
                    <td className="px-4 py-3.5 font-medium sm:px-6">{row.label}</td>
                    <td className="px-4 py-3.5 text-right font-mono text-sm text-primary sm:px-6">{row.betterkv}</td>
                    <td className="px-4 py-3.5 text-right font-mono text-sm text-muted-foreground sm:px-6">{row.redis}</td>
                    <td className="px-4 py-3.5 text-right sm:px-6">
                      <Badge variant="secondary">{row.ratio}</Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </motion.div>
        </div>
      </section>

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <h2 className="text-2xl font-bold tracking-tight">Latency Distribution</h2>
            <p className="mt-2 text-muted-foreground">
              Measured with sustained load. Lower is better.
            </p>
          </motion.div>

          <div className="mt-10 grid gap-4 md:grid-cols-3">
            {latencyData.map((item) => (
              <motion.div
                key={item.label}
                {...fadeUp}
                className="rounded-xl border border-border/50 bg-card p-6"
              >
                <div className="text-sm text-muted-foreground">{item.label}</div>
                <div className="mt-3 flex items-baseline gap-3">
                  <span className="text-3xl font-bold text-primary">{item.betterkv}</span>
                  <span className="text-sm text-muted-foreground">vs {item.redis}</span>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <section className="py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <h2 className="text-2xl font-bold tracking-tight">Architecture</h2>
            <p className="mt-2 text-muted-foreground">
              What makes BetterKV fast at every level.
            </p>
          </motion.div>

          <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {architectureFeatures.map((feature, i) => (
              <motion.div
                key={feature.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ duration: 0.4, delay: i * 0.08 }}
                className="rounded-xl border border-border/50 bg-card p-6"
              >
                <h3 className="text-sm font-semibold">{feature.title}</h3>
                <p className="mt-2 text-sm text-muted-foreground leading-relaxed">{feature.description}</p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}

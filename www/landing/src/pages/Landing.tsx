import { Link } from "react-router-dom"
import { motion } from "framer-motion"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { WaitlistModal } from "@/components/layout/WaitlistModal"
import {
  ZapIcon,
  ArrowLeftRightIcon,
  CpuIcon,
  MemoryStickIcon,
  ShieldCheckIcon,
  ReplaceIcon,
  ArrowRightIcon,
  DatabaseIcon,
  UserIcon,
  BarChart3Icon,
  MailIcon,
  FlagIcon,
  GamepadIcon,
} from "lucide-react"

const features = [
  {
    icon: ZapIcon,
    title: "5-15x Faster",
    description: "Optimized from the ground up for raw throughput and sub-millisecond latency.",
  },
  {
    icon: ArrowLeftRightIcon,
    title: "Redis Compatible",
    description: "Drop-in replacement. Use your existing Redis clients, no code changes needed.",
  },
  {
    icon: CpuIcon,
    title: "Multi-threaded",
    description: "True multi-threaded architecture that scales linearly with CPU cores.",
  },
  {
    icon: MemoryStickIcon,
    title: "Memory Efficient",
    description: "Optimized memory allocator designed to minimize fragmentation and overhead.",
  },
  {
    icon: ShieldCheckIcon,
    title: "Production Ready",
    description: "Built in Rust for memory safety, thread safety, and zero undefined behavior.",
  },
  {
    icon: ReplaceIcon,
    title: "Drop-in Replacement",
    description: "Migrate in minutes. Switch the connection string — everything else stays the same.",
  },
]

const useCases = [
  { icon: DatabaseIcon, title: "Caching", to: "/use-cases/caching", description: "Application & API response caching" },
  { icon: UserIcon, title: "Sessions", to: "/use-cases/sessions", description: "Fast session management" },
  { icon: BarChart3Icon, title: "Analytics", to: "/use-cases/analytics", description: "Real-time counters & leaderboards" },
  { icon: MailIcon, title: "Queues", to: "/use-cases/queues", description: "Pub/Sub & message queues" },
  { icon: FlagIcon, title: "Feature Flags", to: "/use-cases/feature-flags", description: "Toggle features instantly" },
  { icon: GamepadIcon, title: "Gaming", to: "/use-cases/gaming", description: "Leaderboards & game state" },
]

const fadeUp = {
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, margin: "-100px" },
  transition: { duration: 0.5 },
}

export function LandingPage() {
  return (
    <div>
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,oklch(0.65_0.2_280_/_0.08),transparent_60%)]" />
        <div className="relative mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24 lg:py-36">
          <motion.div {...fadeUp} className="max-w-3xl">
            <Badge variant="secondary" className="mb-5 sm:mb-6">
              Now in Early Access
            </Badge>
            <h1 className="text-3xl font-bold tracking-tight sm:text-5xl lg:text-6xl">
              The key-value store
              <br />
              <span className="text-primary">Redis should have been.</span>
            </h1>
            <p className="mt-4 max-w-xl text-base text-muted-foreground sm:mt-6 sm:text-lg">
              BetterKV is a Redis-compatible key-value store built in Rust.
              Multi-threaded, memory-efficient, and 5-15x faster. Drop it in — your code doesn't change.
            </p>
            <div className="mt-6 flex flex-col gap-3 sm:mt-8 sm:flex-row sm:flex-wrap">
              <Button
                size="lg"
                className="w-full sm:w-auto"
                render={<a href="https://docs.betterkv.com/installation" target="_blank" rel="noopener noreferrer" />}
              >
                Get Started
                <ArrowRightIcon className="ml-1 size-4" />
              </Button>
              <WaitlistModal>
                <Button size="lg" variant="outline" className="w-full cursor-pointer sm:w-auto">
                  Join Cloud Waitlist
                </Button>
              </WaitlistModal>
            </div>
          </motion.div>
        </div>
      </section>

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <Badge variant="secondary" className="mb-4">Performance</Badge>
            <h2 className="text-3xl font-bold tracking-tight">Built for speed.</h2>
            <p className="mt-3 max-w-lg text-muted-foreground">
              Every layer of BetterKV is optimized for throughput and low latency, from the network layer to the storage engine.
            </p>
          </motion.div>

          <div className="mt-14 grid gap-6 md:grid-cols-2">
            <motion.div
              {...fadeUp}
              className="rounded-xl border border-border/50 bg-card p-8"
            >
              <div className="mb-6 text-sm font-medium text-muted-foreground">Operations / second</div>
              <div className="space-y-5">
                <div>
                  <div className="mb-2 flex items-center justify-between text-sm">
                    <span className="font-medium text-primary">BetterKV</span>
                    <span className="font-mono text-xs text-muted-foreground">~2,400,000 ops/s</span>
                  </div>
                  <div className="h-3 overflow-hidden rounded-full bg-muted">
                    <motion.div
                      className="h-full rounded-full bg-primary"
                      initial={{ width: 0 }}
                      whileInView={{ width: "95%" }}
                      viewport={{ once: true }}
                      transition={{ duration: 1, ease: "easeOut", delay: 0.2 }}
                    />
                  </div>
                </div>
                <div>
                  <div className="mb-2 flex items-center justify-between text-sm">
                    <span className="font-medium text-muted-foreground">Redis</span>
                    <span className="font-mono text-xs text-muted-foreground">~200,000 ops/s</span>
                  </div>
                  <div className="h-3 overflow-hidden rounded-full bg-muted">
                    <motion.div
                      className="h-full rounded-full bg-muted-foreground/30"
                      initial={{ width: 0 }}
                      whileInView={{ width: "8%" }}
                      viewport={{ once: true }}
                      transition={{ duration: 0.8, ease: "easeOut", delay: 0.4 }}
                    />
                  </div>
                </div>
              </div>
            </motion.div>

            <motion.div
              {...fadeUp}
              className="rounded-xl border border-border/50 bg-card p-8"
            >
              <div className="mb-6 text-sm font-medium text-muted-foreground">Average latency</div>
              <div className="space-y-5">
                <div>
                  <div className="mb-2 flex items-center justify-between text-sm">
                    <span className="font-medium text-primary">BetterKV</span>
                    <span className="font-mono text-xs text-muted-foreground">~8 &micro;s</span>
                  </div>
                  <div className="h-3 overflow-hidden rounded-full bg-muted">
                    <motion.div
                      className="h-full rounded-full bg-primary"
                      initial={{ width: 0 }}
                      whileInView={{ width: "6%" }}
                      viewport={{ once: true }}
                      transition={{ duration: 0.6, ease: "easeOut", delay: 0.2 }}
                    />
                  </div>
                </div>
                <div>
                  <div className="mb-2 flex items-center justify-between text-sm">
                    <span className="font-medium text-muted-foreground">Redis</span>
                    <span className="font-mono text-xs text-muted-foreground">~100 &micro;s</span>
                  </div>
                  <div className="h-3 overflow-hidden rounded-full bg-muted">
                    <motion.div
                      className="h-full rounded-full bg-muted-foreground/30"
                      initial={{ width: 0 }}
                      whileInView={{ width: "75%" }}
                      viewport={{ once: true }}
                      transition={{ duration: 1, ease: "easeOut", delay: 0.4 }}
                    />
                  </div>
                </div>
              </div>
            </motion.div>
          </div>

          <motion.div {...fadeUp} className="mt-6 text-center">
            <Link to="/performance" className="text-sm text-muted-foreground transition-colors hover:text-primary">
              See full benchmarks &rarr;
            </Link>
          </motion.div>
        </div>
      </section>

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <Badge variant="secondary" className="mb-4">Features</Badge>
            <h2 className="text-3xl font-bold tracking-tight">Everything you need.</h2>
            <p className="mt-3 max-w-lg text-muted-foreground">
              A complete key-value store that's faster, safer, and just as easy to use.
            </p>
          </motion.div>

          <div className="mt-14 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {features.map((feature, i) => (
              <motion.div
                key={feature.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ duration: 0.4, delay: i * 0.08 }}
                className="rounded-xl border border-border/50 bg-card p-6 transition-colors hover:border-primary/20"
              >
                <div className="mb-4 flex size-10 items-center justify-center rounded-lg bg-primary/10">
                  <feature.icon className="size-5 text-primary" />
                </div>
                <h3 className="text-sm font-semibold">{feature.title}</h3>
                <p className="mt-1.5 text-sm text-muted-foreground">{feature.description}</p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <Badge variant="secondary" className="mb-4">Compatibility</Badge>
            <h2 className="text-3xl font-bold tracking-tight">Works with your stack.</h2>
            <p className="mt-3 max-w-lg text-muted-foreground">
              Use any Redis client. No SDK changes. Just point it at BetterKV.
            </p>
          </motion.div>

          <motion.div
            {...fadeUp}
            className="mt-14 overflow-hidden rounded-xl border border-border/50 bg-card"
          >
            <div className="flex items-center gap-2 border-b border-border/50 px-4 py-3">
              <div className="size-3 rounded-full bg-muted-foreground/20" />
              <div className="size-3 rounded-full bg-muted-foreground/20" />
              <div className="size-3 rounded-full bg-muted-foreground/20" />
              <span className="ml-2 text-xs text-muted-foreground">terminal</span>
            </div>
            <div className="p-6 font-mono text-sm leading-relaxed">
              <div className="text-muted-foreground">
                <span className="text-primary">$</span> redis-cli -h localhost -p 6380
              </div>
              <div className="mt-3 text-muted-foreground">
                <span className="text-primary">127.0.0.1:6380&gt;</span>{" "}
                <span className="text-foreground">SET user:1 "Jane Doe"</span>
              </div>
              <div className="text-emerald-400">OK</div>
              <div className="mt-2 text-muted-foreground">
                <span className="text-primary">127.0.0.1:6380&gt;</span>{" "}
                <span className="text-foreground">GET user:1</span>
              </div>
              <div className="text-emerald-400">"Jane Doe"</div>
              <div className="mt-2 text-muted-foreground">
                <span className="text-primary">127.0.0.1:6380&gt;</span>{" "}
                <span className="text-foreground">HSET session:abc token "xyz" expires 3600</span>
              </div>
              <div className="text-emerald-400">(integer) 2</div>
              <div className="mt-2 text-muted-foreground">
                <span className="text-primary">127.0.0.1:6380&gt;</span>{" "}
                <span className="text-foreground">PING</span>
              </div>
              <div className="text-emerald-400">PONG</div>
            </div>
          </motion.div>

          <motion.div {...fadeUp} className="mt-6 text-center">
            <Link to="/compatibility" className="text-sm text-muted-foreground transition-colors hover:text-primary">
              See compatibility details &rarr;
            </Link>
          </motion.div>
        </div>
      </section>

      <section className="border-b border-border/50 py-24">
        <div className="mx-auto max-w-6xl px-6">
          <motion.div {...fadeUp}>
            <Badge variant="secondary" className="mb-4">Use Cases</Badge>
            <h2 className="text-3xl font-bold tracking-tight">Built for every workload.</h2>
            <p className="mt-3 max-w-lg text-muted-foreground">
              From caching to real-time analytics, BetterKV handles it all — faster.
            </p>
          </motion.div>

          <div className="mt-14 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {useCases.map((uc, i) => (
              <motion.div
                key={uc.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true, margin: "-50px" }}
                transition={{ duration: 0.4, delay: i * 0.08 }}
              >
                <Link
                  to={uc.to}
                  className="group flex items-start gap-4 rounded-xl border border-border/50 bg-card p-6 transition-colors hover:border-primary/20"
                >
                  <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                    <uc.icon className="size-5 text-primary" />
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold group-hover:text-primary">{uc.title}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{uc.description}</p>
                  </div>
                </Link>
              </motion.div>
            ))}
          </div>

          <motion.div {...fadeUp} className="mt-8 text-center">
            <Link to="/use-cases" className="text-sm text-muted-foreground transition-colors hover:text-primary">
              View all use cases &rarr;
            </Link>
          </motion.div>
        </div>
      </section>

      <section className="relative overflow-hidden py-24">
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_bottom,oklch(0.65_0.2_280_/_0.06),transparent_60%)]" />
        <div className="relative mx-auto max-w-6xl px-6 text-center">
          <motion.div {...fadeUp}>
            <Badge variant="secondary" className="mb-4">Coming Soon</Badge>
            <h2 className="text-3xl font-bold tracking-tight">BetterKV Cloud</h2>
            <p className="mx-auto mt-3 max-w-md text-muted-foreground">
              Managed, hosted, and ready to scale. Join the waitlist to get early access
              and be first to know when we launch.
            </p>
            <div className="mt-8">
              <WaitlistModal>
                <Button size="lg" className="cursor-pointer">
                  Join the Waitlist
                  <ArrowRightIcon className="ml-1 size-4" />
                </Button>
              </WaitlistModal>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  )
}

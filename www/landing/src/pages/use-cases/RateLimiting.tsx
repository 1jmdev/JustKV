import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    ShieldCheckIcon,
    ClockIcon,
    GlobeIcon,
    ZapIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: ShieldCheckIcon,
        title: "Protect Your APIs",
        description:
            "Prevent abuse and enforce fair usage with precise, per-key rate limiting. Every decision in microseconds.",
    },
    {
        icon: ClockIcon,
        title: "Sliding Window",
        description:
            "Implement sliding window rate limiters with atomic operations. No race conditions, even under massive concurrent load.",
    },
    {
        icon: GlobeIcon,
        title: "Distributed Rate Limiting",
        description:
            "Share rate limit state across all your application servers. Consistent enforcement regardless of which node handles the request.",
    },
    {
        icon: ZapIcon,
        title: "Zero Overhead",
        description:
            "Rate limit checks complete in single-digit microseconds. Your users won't notice the overhead — but attackers will notice the wall.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function RateLimitingPage() {
    return (
        <div>
            <Seo
                title="Distributed Rate Limiting with BetterKV"
                description="Implement microsecond-decision rate limiting across distributed systems using BetterKV. Redis-compatible sliding window and token bucket patterns at massive scale."
                path="/use-cases/rate-limiting"
            />
            <PageHeader
                badge="Use Case"
                title="Rate Limiting"
                description="Distributed rate limiting with microsecond decisions. Protect your APIs without adding latency."
            />

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <div className="grid gap-4 sm:grid-cols-2">
                        {benefits.map((benefit, i) => (
                            <motion.div
                                key={benefit.title}
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true, margin: "-50px" }}
                                transition={{ duration: 0.4, delay: i * 0.08 }}
                                className="rounded-xl border border-border/50 bg-card p-6"
                            >
                                <div className="mb-4 flex size-10 items-center justify-center rounded-lg bg-primary/10">
                                    <benefit.icon className="size-5 text-primary" />
                                </div>
                                <h3 className="text-sm font-semibold">
                                    {benefit.title}
                                </h3>
                                <p className="mt-2 text-sm text-muted-foreground leading-relaxed">
                                    {benefit.description}
                                </p>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Example: Sliding Window Rate Limiter
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            A simple, atomic rate limiter using INCR and EXPIRE.
                        </p>
                    </motion.div>

                    <motion.div
                        {...fadeUp}
                        className="mt-8 overflow-hidden rounded-xl border border-border/50 bg-card"
                    >
                        <div className="flex items-center gap-2 border-b border-border/50 px-4 py-3">
                            <div className="size-3 rounded-full bg-muted-foreground/20" />
                            <div className="size-3 rounded-full bg-muted-foreground/20" />
                            <div className="size-3 rounded-full bg-muted-foreground/20" />
                            <span className="ml-2 text-xs text-muted-foreground">
                                ratelimit.ts
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`import { createClient } from 'redis'

const client = createClient({ url: 'redis://localhost:6380' })

async function isRateLimited(ip: string, limit = 100) {
  const key = \`rate:\${ip}:\${Math.floor(Date.now() / 60000)}\`
  
  const count = await client.incr(key)
  
  if (count === 1) {
    await client.expire(key, 60)
  }
  
  return count > limit
}`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Rate limiting that doesn't slow you down.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            Protect your APIs with BetterKV. Microsecond
                            decisions at any scale.
                        </p>
                        <div className="mt-8">
                            <Button
                                size="lg"
                                render={
                                    <a
                                        href="https://docs.betterkv.com/installation"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    />
                                }
                            >
                                Get Started
                                <ArrowRightIcon className="ml-1 size-4" />
                            </Button>
                        </div>
                    </motion.div>
                </div>
            </section>
        </div>
    );
}

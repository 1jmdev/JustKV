import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    BarChart3Icon,
    TimerIcon,
    TrendingUpIcon,
    LayersIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: BarChart3Icon,
        title: "Real-time Counters",
        description:
            "Atomic INCR operations at millions per second. Track page views, API calls, events, or any metric in real time.",
    },
    {
        icon: TrendingUpIcon,
        title: "Live Leaderboards",
        description:
            "Sorted Sets give you instant leaderboards with O(log N) inserts and O(1) rank lookups. Always up to date.",
    },
    {
        icon: TimerIcon,
        title: "Time-series Data",
        description:
            "Use sorted sets with timestamps to build time-series storage. Query any time range in microseconds.",
    },
    {
        icon: LayersIcon,
        title: "Aggregation Pipelines",
        description:
            "Pipeline multiple operations in a single round-trip. Compute complex aggregations without network overhead.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function AnalyticsPage() {
    return (
        <div>
            <Seo
                title="Real-Time Analytics with BetterKV — Counters, Leaderboards & Time-Series"
                description="Power real-time dashboards, counters, and leaderboards with BetterKV. Millions of increments per second, sorted sets for rankings, and Redis-compatible commands."
                path="/use-cases/analytics"
            />
            <PageHeader
                badge="Use Case"
                title="Real-time Analytics"
                description="Counters, leaderboards, and time-series data at the speed your dashboards demand."
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
                            Example: Live Leaderboard
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Sorted Sets make leaderboards trivial. Millions of
                            entries, instant rank lookups.
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
                                terminal
                            </span>
                        </div>
                        <div className="p-6 font-mono text-sm leading-relaxed">
                            <div className="text-muted-foreground">
                                <span className="text-primary">$</span>{" "}
                                redis-cli -p 6380
                            </div>
                            <div className="mt-3 text-muted-foreground">
                                <span className="text-primary">&gt;</span>{" "}
                                <span className="text-foreground">
                                    ZADD leaderboard 1500 "player:alice"
                                </span>
                            </div>
                            <div className="text-emerald-400">(integer) 1</div>
                            <div className="mt-2 text-muted-foreground">
                                <span className="text-primary">&gt;</span>{" "}
                                <span className="text-foreground">
                                    ZADD leaderboard 2100 "player:bob"
                                </span>
                            </div>
                            <div className="text-emerald-400">(integer) 1</div>
                            <div className="mt-2 text-muted-foreground">
                                <span className="text-primary">&gt;</span>{" "}
                                <span className="text-foreground">
                                    ZINCRBY leaderboard 300 "player:alice"
                                </span>
                            </div>
                            <div className="text-emerald-400">"1800"</div>
                            <div className="mt-2 text-muted-foreground">
                                <span className="text-primary">&gt;</span>{" "}
                                <span className="text-foreground">
                                    ZREVRANGE leaderboard 0 9 WITHSCORES
                                </span>
                            </div>
                            <div className="text-emerald-400">
                                1) "player:bob"
                            </div>
                            <div className="text-emerald-400">2) "2100"</div>
                            <div className="text-emerald-400">
                                3) "player:alice"
                            </div>
                            <div className="text-emerald-400">4) "1800"</div>
                        </div>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Real-time data deserves real-time infrastructure.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            BetterKV processes millions of atomic updates per
                            second. Your dashboards will thank you.
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

import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    GamepadIcon,
    TrophyIcon,
    UsersIcon,
    ZapIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: TrophyIcon,
        title: "Real-time Leaderboards",
        description:
            "Sorted Sets give you instant leaderboards with O(log N) updates and O(1) rank lookups. Millions of players, zero lag.",
    },
    {
        icon: UsersIcon,
        title: "Matchmaking",
        description:
            "Store player ratings and find matches in microseconds. Sorted Sets make range-based matchmaking trivial.",
    },
    {
        icon: GamepadIcon,
        title: "Game State",
        description:
            "Store active game sessions, player inventories, and world state in memory. Instant reads and writes for real-time gameplay.",
    },
    {
        icon: ZapIcon,
        title: "Low Latency Multiplayer",
        description:
            "Sub-10 microsecond operations mean your game server never waits on the data layer. BetterKV keeps up with your tick rate.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function GamingPage() {
    return (
        <div>
            <Seo
                title="Gaming Infrastructure with BetterKV — Leaderboards & Matchmaking"
                description="Power gaming leaderboards, matchmaking queues, and real-time game state with BetterKV. Sorted sets and pub/sub at the latency your players expect."
                path="/use-cases/gaming"
            />
            <PageHeader
                badge="Use Case"
                title="Gaming"
                description="Leaderboards, matchmaking, and game state at the speed your players demand."
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
                            Example: Global Leaderboard
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Maintain a real-time leaderboard for millions of
                            players.
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
                                leaderboard.go
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`package main

import "github.com/redis/go-redis/v9"

var rdb = redis.NewClient(&redis.Options{
    Addr: "localhost:6380",
})

func UpdateScore(player string, score float64) {
    rdb.ZAdd(ctx, "leaderboard:global", redis.Z{
        Score:  score,
        Member: player,
    })
}

func GetTopPlayers(count int) []redis.Z {
    return rdb.ZRevRangeWithScores(
        ctx, "leaderboard:global", 0, int64(count-1),
    ).Val()
}

func GetRank(player string) int64 {
    return rdb.ZRevRank(ctx, "leaderboard:global", player).Val()
}`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Your data layer shouldn't be the bottleneck.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            BetterKV keeps up with your game server's tick rate.
                            Every time.
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

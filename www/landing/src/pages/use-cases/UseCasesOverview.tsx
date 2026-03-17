import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import {
    ZapIcon,
    UserIcon,
    BarChart3Icon,
    MailIcon,
    DatabaseIcon,
    FlagIcon,
    GamepadIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const useCases = [
    {
        icon: ZapIcon,
        title: "Caching",
        to: "/use-cases/caching",
        description:
            "Accelerate your application with sub-millisecond cache lookups. Reduce database load by 90%+ and serve responses 10x faster.",
    },
    {
        icon: UserIcon,
        title: "Session Storage",
        to: "/use-cases/sessions",
        description:
            "Store and retrieve user sessions at scale. Handle millions of concurrent sessions with microsecond access times.",
    },
    {
        icon: BarChart3Icon,
        title: "Real-time Analytics",
        to: "/use-cases/analytics",
        description:
            "Power real-time dashboards, counters, leaderboards, and time-series data with instant reads and atomic updates.",
    },
    {
        icon: MailIcon,
        title: "Message Queues",
        to: "/use-cases/queues",
        description:
            "Build reliable pub/sub systems and task queues. Process millions of messages per second with guaranteed ordering.",
    },
    {
        icon: DatabaseIcon,
        title: "Rate Limiting",
        to: "/use-cases/rate-limiting",
        description:
            "Protect your APIs with precise, distributed rate limiting. Sub-microsecond decisions at any scale.",
    },
    {
        icon: FlagIcon,
        title: "Feature Flags",
        to: "/use-cases/feature-flags",
        description:
            "Toggle features instantly across your entire infrastructure. Zero-latency flag evaluation for every request.",
    },
    {
        icon: GamepadIcon,
        title: "Gaming",
        to: "/use-cases/gaming",
        description:
            "Real-time leaderboards, matchmaking, and game state. Handle millions of concurrent players without breaking a sweat.",
    },
];

export function UseCasesOverviewPage() {
    return (
        <div>
            <Seo
                title="Use Cases — BetterKV"
                description="BetterKV powers caching, session storage, real-time analytics, message queues, rate limiting, feature flags, and gaming leaderboards. 5–30x faster than Redis."
                path="/use-cases"
            />
            <PageHeader
                badge="Use Cases"
                title="Built for every workload."
                description="From caching to real-time analytics, BetterKV handles it all — 5-15x faster than Redis."
            />

            <section className="py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
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
                                    className="group flex h-full flex-col rounded-xl border border-border/50 bg-card p-6 transition-colors hover:border-primary/20"
                                >
                                    <div className="mb-4 flex size-10 items-center justify-center rounded-lg bg-primary/10">
                                        <uc.icon className="size-5 text-primary" />
                                    </div>
                                    <h3 className="text-sm font-semibold group-hover:text-primary">
                                        {uc.title}
                                    </h3>
                                    <p className="mt-2 flex-1 text-sm text-muted-foreground leading-relaxed">
                                        {uc.description}
                                    </p>
                                    <span className="mt-4 text-sm text-primary opacity-0 transition-opacity group-hover:opacity-100">
                                        Learn more &rarr;
                                    </span>
                                </Link>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}

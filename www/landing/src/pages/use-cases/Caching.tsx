import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    ZapIcon,
    DatabaseIcon,
    GlobeIcon,
    ServerIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: ZapIcon,
        title: "Sub-microsecond Reads",
        description:
            "Cache hits return in under 8 microseconds. Your users get instant responses, your databases get a break.",
    },
    {
        icon: DatabaseIcon,
        title: "90%+ Database Offload",
        description:
            "Move hot data to BetterKV and watch your database CPU drop. Most applications see a 90-95% reduction in database queries.",
    },
    {
        icon: GlobeIcon,
        title: "Application Cache",
        description:
            "Cache API responses, rendered pages, computed results, and any data that's expensive to generate or fetch.",
    },
    {
        icon: ServerIcon,
        title: "Multi-tier Caching",
        description:
            "Use BetterKV as your L1 cache alongside your existing CDN. Hot data stays in memory, everything else falls through.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function CachingPage() {
    return (
        <div>
            <Seo
                title="High-Performance Caching with BetterKV"
                description="Replace Redis as your cache layer with BetterKV. Sub-microsecond reads, millions of GET/SET operations per second, and full Redis client compatibility."
                path="/use-cases/caching"
            />
            <PageHeader
                badge="Use Case"
                title="Caching"
                description="Accelerate your application with the fastest cache layer available. Sub-microsecond reads, millions of operations per second."
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
                            Example: API Response Cache
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Cache expensive API responses with automatic
                            expiration.
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
                                app.py
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`import redis

cache = redis.Redis(host='localhost', port=6380)

def get_user(user_id):
    key = f"user:{user_id}"
    
    cached = cache.get(key)
    if cached:
        return json.loads(cached)
    
    user = db.query("SELECT * FROM users WHERE id = %s", user_id)
    
    cache.setex(key, 300, json.dumps(user))
    
    return user`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Ready to speed up your application?
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            BetterKV is a drop-in replacement for Redis. Install
                            it and point your cache at it.
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

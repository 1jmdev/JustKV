import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    UserIcon,
    ClockIcon,
    ShieldCheckIcon,
    ServerIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: UserIcon,
        title: "Millions of Concurrent Sessions",
        description:
            "Handle millions of active user sessions without breaking a sweat. Each session lookup completes in microseconds.",
    },
    {
        icon: ClockIcon,
        title: "Automatic Expiration",
        description:
            "Sessions expire automatically with TTL support. No background cleanup jobs, no stale data accumulating over time.",
    },
    {
        icon: ShieldCheckIcon,
        title: "Secure by Default",
        description:
            "TLS encryption, authentication, and ACL support ensure session data is protected at every layer.",
    },
    {
        icon: ServerIcon,
        title: "Distributed Sessions",
        description:
            "Share sessions across multiple application servers. Stateless backends become trivial when your session store is this fast.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function SessionsPage() {
    return (
        <div>
            <Seo
                title="Session Storage with BetterKV — Fast, Scalable, Redis-Compatible"
                description="Manage millions of concurrent user sessions with microsecond access times. BetterKV is a drop-in Redis replacement for high-scale session storage."
                path="/use-cases/sessions"
            />
            <PageHeader
                badge="Use Case"
                title="Session Storage"
                description="Fast, reliable session management at scale. Microsecond access times for millions of concurrent users."
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
                            Example: Express Session Store
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Use BetterKV as your session backend with any
                            Redis-compatible session middleware.
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
                                server.js
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`import session from 'express-session'
import RedisStore from 'connect-redis'
import { createClient } from 'redis'

const client = createClient({ url: 'redis://localhost:6380' })
await client.connect()

app.use(session({
  store: new RedisStore({ client }),
  secret: 'your-secret',
  resave: false,
  saveUninitialized: false,
  cookie: { maxAge: 86400000 }
}))`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Faster sessions, happier users.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            Drop BetterKV in as your session store and cut
                            authentication latency by 10x.
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

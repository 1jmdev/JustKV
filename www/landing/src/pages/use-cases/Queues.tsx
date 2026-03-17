import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    MailIcon,
    RadioIcon,
    ListIcon,
    ZapIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: RadioIcon,
        title: "Pub/Sub Messaging",
        description:
            "Publish messages to channels and have all subscribers receive them instantly. Pattern-based subscriptions included.",
    },
    {
        icon: ListIcon,
        title: "Reliable Task Queues",
        description:
            "Use LPUSH/BRPOP for simple, reliable work queues. Tasks are processed in order with at-least-once delivery.",
    },
    {
        icon: ZapIcon,
        title: "Millions of Messages/sec",
        description:
            "BetterKV's multi-threaded engine handles millions of pub/sub messages per second without breaking a sweat.",
    },
    {
        icon: MailIcon,
        title: "Fan-out Patterns",
        description:
            "Broadcast events to hundreds of subscribers simultaneously. Real-time notifications, webhooks, and event-driven architectures.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function QueuesPage() {
    return (
        <div>
            <Seo
                title="Message Queues & Pub/Sub with BetterKV"
                description="Build high-throughput task queues and pub/sub messaging systems with BetterKV. Millions of messages per second using Redis-compatible LIST and SUBSCRIBE commands."
                path="/use-cases/queues"
            />
            <PageHeader
                badge="Use Case"
                title="Message Queues"
                description="Pub/Sub messaging and task queues with millions of messages per second throughput."
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
                            Example: Event Broadcasting
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Publish events and have all connected services react
                            in real time.
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
                                publisher.js
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`import { createClient } from 'redis'

const publisher = createClient({ url: 'redis://localhost:6380' })
await publisher.connect()

await publisher.publish('orders', JSON.stringify({
  event: 'order.created',
  orderId: '12345',
  amount: 99.99,
  timestamp: Date.now()
}))`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Event-driven architecture, zero friction.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            Same Redis pub/sub API. 10x the throughput.
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

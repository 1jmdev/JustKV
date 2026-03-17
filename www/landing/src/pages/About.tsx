import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Seo } from "@/components/Seo";

const values = [
    {
        title: "Performance First",
        description:
            "Every decision we make is measured against its impact on latency and throughput. If it makes things slower, it doesn't ship.",
    },
    {
        title: "Simplicity",
        description:
            "BetterKV should be easy to install, easy to configure, and easy to operate. No PhD required.",
    },
    {
        title: "Compatibility",
        description:
            "We don't break your existing code. Redis compatibility isn't an afterthought — it's a core design constraint.",
    },
    {
        title: "Transparency",
        description:
            "Open source, public roadmap, honest benchmarks. We show our work.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function AboutPage() {
    return (
        <div>
            <Seo
                title="About — BetterKV"
                description="BetterKV is an open-source Redis-compatible key-value store built in Rust from the ground up for low tail latency, high throughput, and operational simplicity."
                path="/about"
            />
            <PageHeader
                badge="About"
                title="We're building the future of in-memory data."
                description="BetterKV started with a simple question: what if Redis was built today, with modern hardware and modern tools?"
            />

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div
                        {...fadeUp}
                        className="space-y-6 text-muted-foreground leading-relaxed"
                    >
                        <p>
                            Redis is one of the most important pieces of
                            infrastructure in modern software. It's fast,
                            simple, and everywhere. But its core architecture
                            was designed over 15 years ago for single-threaded,
                            single-core machines.
                        </p>
                        <p>
                            Modern servers have 64, 128, even 256 cores. Memory
                            is cheap and abundant. Network cards can push 100
                            Gbps. Redis can't take advantage of any of this
                            because of fundamental architectural decisions that
                            can't be changed without a complete rewrite.
                        </p>
                        <p>
                            So we did the rewrite. BetterKV is built from
                            scratch in Rust with a thread-per-core architecture,
                            lock-free data structures, and io_uring-based
                            networking. The result is a Redis-compatible
                            key-value store that's 5-15x faster while remaining
                            a true drop-in replacement.
                        </p>
                        <p>
                            We believe infrastructure software should be fast,
                            reliable, and simple. That's what we're building.
                        </p>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Our Values
                        </h2>
                    </motion.div>

                    <div className="mt-10 grid gap-4 sm:grid-cols-2">
                        {values.map((value, i) => (
                            <motion.div
                                key={value.title}
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true, margin: "-50px" }}
                                transition={{ duration: 0.4, delay: i * 0.08 }}
                                className="rounded-xl border border-border/50 bg-card p-6"
                            >
                                <h3 className="text-sm font-semibold">
                                    {value.title}
                                </h3>
                                <p className="mt-2 text-sm text-muted-foreground leading-relaxed">
                                    {value.description}
                                </p>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}

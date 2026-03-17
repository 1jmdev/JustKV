import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Seo } from "@/components/Seo";

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function RoadmapPage() {
    return (
        <div>
            <Seo
                title="Roadmap — BetterKV"
                description="See what's coming next for BetterKV — an open-source Redis-compatible key-value store built in Rust. Follow development on GitHub."
                path="/roadmap"
            />
            <PageHeader
                badge="Roadmap"
                title="Coming soon."
                description="A public roadmap is on the way. BetterKV is currently being developed in the open on GitHub."
            />

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div
                        {...fadeUp}
                        className="rounded-xl border border-border/50 bg-card p-8"
                    >
                        <h2 className="text-2xl font-bold tracking-tight">
                            Public roadmap coming soon
                        </h2>
                        <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                            We are not publishing a detailed roadmap on the
                            landing site yet. For now, BetterKV is in open beta
                            and active development happens publicly on GitHub.
                        </p>
                        <p className="mt-4 text-sm leading-relaxed text-muted-foreground">
                            Track progress here: {" "}
                            <a
                                href="https://github.com/1jmdev/BetterKV"
                                target="_blank"
                                rel="noreferrer"
                                className="text-primary underline underline-offset-4"
                            >
                                github.com/1jmdev/BetterKV
                            </a>
                        </p>
                    </motion.div>
                </div>
            </section>
        </div>
    );
}

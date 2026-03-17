import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Seo } from "@/components/Seo";

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function ChangelogPage() {
    return (
        <div>
            <Seo
                title="Changelog — BetterKV"
                description="Release notes and version history for BetterKV, the high-performance Redis-compatible key-value store built in Rust."
                path="/changelog"
            />
            <PageHeader
                badge="Changelog"
                title="Coming soon."
                description="We have not published formal releases yet. BetterKV is currently in open beta on GitHub."
            />

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div
                        {...fadeUp}
                        className="rounded-xl border border-border/50 bg-card p-8"
                    >
                        <h2 className="text-2xl font-bold tracking-tight">
                            No releases yet
                        </h2>
                        <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                            A public changelog is coming soon. For now, BetterKV
                            is available as an open beta on GitHub.
                        </p>
                        <p className="mt-4 text-sm leading-relaxed text-muted-foreground">
                            Follow development here: {" "}
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

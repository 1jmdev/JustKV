import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    FlagIcon,
    ZapIcon,
    UsersIcon,
    ToggleLeftIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const benefits = [
    {
        icon: FlagIcon,
        title: "Instant Toggling",
        description:
            "Enable or disable features across your entire infrastructure with a single command. Changes propagate in microseconds.",
    },
    {
        icon: ZapIcon,
        title: "Zero-latency Evaluation",
        description:
            "Flag checks complete in single-digit microseconds. Evaluate flags on every request without measurable overhead.",
    },
    {
        icon: UsersIcon,
        title: "Per-user Targeting",
        description:
            "Use hashes to store per-user or per-segment flag values. Roll out features gradually with fine-grained control.",
    },
    {
        icon: ToggleLeftIcon,
        title: "Simple Data Model",
        description:
            "Flags are just keys. Use SET for global flags, HSET for targeted flags. No complex SDKs or configuration files needed.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function FeatureFlagsPage() {
    return (
        <div>
            <Seo
                title="Feature Flags with BetterKV — Zero-Latency Flag Evaluation"
                description="Store and evaluate feature flags with sub-millisecond latency using BetterKV. Redis-compatible hash and string operations for instant feature toggling across your infrastructure."
                path="/use-cases/feature-flags"
            />
            <PageHeader
                badge="Use Case"
                title="Feature Flags"
                description="Toggle features instantly across your infrastructure. Zero-latency evaluation on every request."
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
                            Example: Feature Flag System
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Simple, fast feature flags with BetterKV.
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
                                flags.ts
                            </span>
                        </div>
                        <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
                            <code>{`import { createClient } from 'redis'

const client = createClient({ url: 'redis://localhost:6380' })

async function isEnabled(flag: string, userId?: string) {
  if (userId) {
    const userFlag = await client.hGet(\`flags:\${flag}\`, userId)
    if (userFlag !== null) return userFlag === '1'
  }
  
  const global = await client.get(\`flag:\${flag}\`)
  return global === '1'
}

await client.set('flag:dark-mode', '1')

await client.hSet('flags:beta-ui', {
  'user:100': '1',
  'user:200': '1',
})`}</code>
                        </pre>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6 text-center">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Ship faster with instant feature flags.
                        </h2>
                        <p className="mx-auto mt-3 max-w-md text-muted-foreground">
                            No heavy SDKs. No external services. Just BetterKV.
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

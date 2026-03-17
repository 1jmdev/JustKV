import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { WaitlistModal } from "@/components/layout/WaitlistModal";
import { CheckIcon, ArrowRightIcon } from "lucide-react";
import { Seo } from "@/components/Seo";

const plans = [
    {
        name: "Self-Hosted",
        price: "Free",
        priceDetail: "forever",
        description:
            "Run BetterKV on your own infrastructure. Full access, no limits.",
        cta: "Download",
        ctaHref: "https://docs.betterkv.com/installation",
        ctaVariant: "outline" as const,
        features: [
            "All data types & commands",
            "Multi-threaded engine",
            "TLS support",
            "Persistence (RDB & AOF)",
            "Pub/Sub",
            "Transactions",
            "Configuration file support",
            "Community support",
        ],
    },
    {
        name: "Cloud",
        price: "Coming Soon",
        priceDetail: null,
        description:
            "Managed BetterKV. We handle the infrastructure so you can focus on building.",
        cta: "Join Waitlist",
        ctaVariant: "default" as const,
        highlighted: true,
        features: [
            "Everything in Self-Hosted",
            "Managed infrastructure",
            "Auto-scaling",
            "Multi-region deployment",
            "Automated backups",
            "Monitoring dashboard",
            "99.99% uptime SLA",
            "Priority support",
            "Usage-based pricing",
        ],
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function PricingPage() {
    return (
        <div>
            <Seo
                title="Pricing — BetterKV"
                description="BetterKV is free to self-host forever. BetterKV Cloud is coming soon — fully managed Redis-compatible key-value store with sub-millisecond latency."
                path="/pricing"
            />
            <PageHeader
                badge="Pricing"
                title="Simple, transparent pricing."
                description="Self-host for free. Or let us handle it when BetterKV Cloud launches."
            />

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <div className="grid gap-6 md:grid-cols-2">
                        {plans.map((plan) => (
                            <motion.div
                                key={plan.name}
                                {...fadeUp}
                                className={`rounded-xl border bg-card p-8 ${
                                    plan.highlighted
                                        ? "border-primary/30 glow-purple-sm"
                                        : "border-border/50"
                                }`}
                            >
                                <div className="flex items-center gap-3">
                                    <h2 className="text-lg font-semibold">
                                        {plan.name}
                                    </h2>
                                    {plan.highlighted && <Badge>Popular</Badge>}
                                </div>

                                <div className="mt-4">
                                    <span className="text-3xl font-bold">
                                        {plan.price}
                                    </span>
                                    {plan.priceDetail && (
                                        <span className="ml-1 text-sm text-muted-foreground">
                                            {plan.priceDetail}
                                        </span>
                                    )}
                                </div>

                                <p className="mt-2 text-sm text-muted-foreground">
                                    {plan.description}
                                </p>

                                <div className="mt-6">
                                    {plan.ctaHref ? (
                                        <Button
                                            variant={plan.ctaVariant}
                                            className="w-full"
                                            render={
                                                <a
                                                    href={plan.ctaHref}
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                />
                                            }
                                        >
                                            {plan.cta}
                                            <ArrowRightIcon className="ml-1 size-4" />
                                        </Button>
                                    ) : (
                                        <WaitlistModal>
                                            <Button
                                                variant={plan.ctaVariant}
                                                className="w-full cursor-pointer"
                                            >
                                                {plan.cta}
                                                <ArrowRightIcon className="ml-1 size-4" />
                                            </Button>
                                        </WaitlistModal>
                                    )}
                                </div>

                                <ul className="mt-8 space-y-3">
                                    {plan.features.map((feature) => (
                                        <li
                                            key={feature}
                                            className="flex items-center gap-2.5 text-sm"
                                        >
                                            <CheckIcon className="size-4 shrink-0 text-primary" />
                                            <span className="text-muted-foreground">
                                                {feature}
                                            </span>
                                        </li>
                                    ))}
                                </ul>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}

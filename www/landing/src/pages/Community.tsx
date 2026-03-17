import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
    ArrowRightIcon,
    GitBranchIcon,
    BookOpenIcon,
    MessageSquareIcon,
    HeartIcon,
} from "lucide-react";
import { Seo } from "@/components/Seo";

const channels = [
    {
        icon: GitBranchIcon,
        title: "GitHub",
        description:
            "Browse the source code, open issues, submit pull requests, and track development progress.",
        cta: "View Repository",
        href: "https://github.com/1jmdev/BetterKV",
    },
    {
        icon: BookOpenIcon,
        title: "Documentation",
        description:
            "Comprehensive guides, API reference, and tutorials to help you get the most out of BetterKV.",
        cta: "Read the Docs",
        href: "https://docs.betterkv.com",
    },
    {
        icon: MessageSquareIcon,
        title: "GitHub Discussions",
        description:
            "Ask questions, share ideas, and connect with other BetterKV users and contributors.",
        cta: "Join Discussions",
        href: "https://github.com/1jmdev/BetterKV/discussions",
    },
];

const contributionSteps = [
    {
        step: 1,
        title: "Fork the Repository",
        description:
            "Create your own fork of BetterKV on GitHub to start making changes.",
    },
    {
        step: 2,
        title: "Pick an Issue",
        description:
            "Browse open issues labeled 'good first issue' or 'help wanted' for a starting point.",
    },
    {
        step: 3,
        title: "Submit a PR",
        description:
            "Make your changes, write tests, and submit a pull request. We review PRs promptly.",
    },
    {
        step: 4,
        title: "Get Reviewed & Merged",
        description:
            "Our team will review your code, provide feedback, and merge once everything looks good.",
    },
];

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function CommunityPage() {
    return (
        <div>
            <Seo
                title="Community — BetterKV"
                description="BetterKV is open source. Contribute on GitHub, read the docs, or join the discussion. Built in the open, for developers."
                path="/community"
            />
            <PageHeader
                badge="Community"
                title="Built in the open."
                description="BetterKV is open source. Join the community, contribute code, or just say hello."
            />

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <div className="grid gap-4 md:grid-cols-3">
                        {channels.map((channel, i) => (
                            <motion.div
                                key={channel.title}
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true, margin: "-50px" }}
                                transition={{ duration: 0.4, delay: i * 0.08 }}
                                className="flex flex-col rounded-xl border border-border/50 bg-card p-6"
                            >
                                <div className="mb-4 flex size-10 items-center justify-center rounded-lg bg-primary/10">
                                    <channel.icon className="size-5 text-primary" />
                                </div>
                                <h3 className="text-sm font-semibold">
                                    {channel.title}
                                </h3>
                                <p className="mt-2 flex-1 text-sm text-muted-foreground leading-relaxed">
                                    {channel.description}
                                </p>
                                <div className="mt-4">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        render={
                                            <a
                                                href={channel.href}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                            />
                                        }
                                    >
                                        {channel.cta}
                                        <ArrowRightIcon className="ml-1 size-3" />
                                    </Button>
                                </div>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div {...fadeUp}>
                        <div className="flex items-center gap-3">
                            <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                                <HeartIcon className="size-5 text-primary" />
                            </div>
                            <h2 className="text-2xl font-bold tracking-tight">
                                Contributing
                            </h2>
                        </div>
                        <p className="mt-3 text-muted-foreground">
                            We welcome contributions of all kinds — code,
                            documentation, bug reports, and feature ideas.
                        </p>
                    </motion.div>

                    <div className="mt-10 grid gap-4 sm:grid-cols-2">
                        {contributionSteps.map((item, i) => (
                            <motion.div
                                key={item.step}
                                initial={{ opacity: 0, y: 20 }}
                                whileInView={{ opacity: 1, y: 0 }}
                                viewport={{ once: true, margin: "-50px" }}
                                transition={{ duration: 0.4, delay: i * 0.08 }}
                                className="rounded-xl border border-border/50 bg-card p-6"
                            >
                                <div className="flex items-center gap-3 mb-3">
                                    <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
                                        {item.step}
                                    </div>
                                    <h3 className="text-sm font-semibold">
                                        {item.title}
                                    </h3>
                                </div>
                                <p className="text-sm text-muted-foreground leading-relaxed">
                                    {item.description}
                                </p>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}

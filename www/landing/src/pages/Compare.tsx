import { motion } from "framer-motion";
import { PageHeader } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
import { CheckIcon, XIcon, MinusIcon } from "lucide-react";
import { Seo } from "@/components/Seo";

const features = [
    {
        feature: "Language",
        betterkv: "Rust",
        redis: "C",
        dragonfly: "C++",
        keydb: "C++",
    },
    {
        feature: "Threading Model",
        betterkv: "Thread-per-core",
        redis: "Single-threaded",
        dragonfly: "Multi-threaded",
        keydb: "Multi-threaded",
    },
    {
        feature: "Redis Protocol",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "RESP3 Support",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: false,
    },
    {
        feature: "Memory Safety",
        betterkv: true,
        redis: false,
        dragonfly: false,
        keydb: false,
    },
    {
        feature: "Lock-free Core",
        betterkv: true,
        redis: "N/A",
        dragonfly: "Partial",
        keydb: false,
    },
    {
        feature: "io_uring",
        betterkv: true,
        redis: false,
        dragonfly: true,
        keydb: false,
    },
    {
        feature: "TLS Support",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Persistence (RDB)",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Persistence (AOF)",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Pub/Sub",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Transactions",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Lua Scripting",
        betterkv: "Planned",
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Cluster Mode",
        betterkv: "Planned",
        redis: true,
        dragonfly: true,
        keydb: true,
    },
    {
        feature: "Active Development",
        betterkv: true,
        redis: true,
        dragonfly: true,
        keydb: "Slow",
    },
    {
        feature: "Open Source",
        betterkv: true,
        redis: true,
        dragonfly: "BSL",
        keydb: true,
    },
];

const throughput = [
    { product: "BetterKV", ops: "~2,400,000", highlight: true },
    { product: "Dragonfly", ops: "~1,000,000", highlight: false },
    { product: "KeyDB", ops: "~500,000", highlight: false },
    { product: "Redis", ops: "~200,000", highlight: false },
];

function CellValue({ value }: { value: boolean | string }) {
    if (value === true)
        return <CheckIcon className="mx-auto size-4 text-emerald-400" />;
    if (value === false)
        return <XIcon className="mx-auto size-4 text-muted-foreground/40" />;
    if (value === "N/A")
        return (
            <MinusIcon className="mx-auto size-4 text-muted-foreground/40" />
        );
    if (
        value === "Partial" ||
        value === "Planned" ||
        value === "Slow" ||
        value === "BSL"
    ) {
        return <span className="text-amber-400">{value}</span>;
    }
    return <span>{value}</span>;
}

const fadeUp = {
    initial: { opacity: 0, y: 20 },
    whileInView: { opacity: 1, y: 0 },
    viewport: { once: true, margin: "-100px" },
    transition: { duration: 0.5 },
};

export function ComparePage() {
    return (
        <div>
            <Seo
                title="BetterKV vs Redis vs Dragonfly vs KeyDB"
                description="Compare BetterKV against Redis, Dragonfly, and KeyDB on performance, threading model, Redis compatibility, memory usage, licensing, and operational complexity."
                path="/compare"
            />
            <PageHeader
                badge="Compare"
                title="BetterKV vs the competition."
                description="An honest look at how BetterKV stacks up against Redis, Dragonfly, and KeyDB."
            />

            <section className="border-b border-border/50 py-24">
                <div className="mx-auto max-w-6xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Feature Comparison
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            Side-by-side feature comparison across key-value
                            stores.
                        </p>
                    </motion.div>

                    <motion.div
                        {...fadeUp}
                        className="mt-10 rounded-xl border border-border/50 overflow-x-auto"
                    >
                        <table className="w-full min-w-140 text-sm">
                            <thead>
                                <tr className="border-b border-border/50 bg-card">
                                    <th className="px-4 py-3.5 text-left font-medium text-muted-foreground sm:px-5">
                                        Feature
                                    </th>
                                    <th className="px-4 py-3.5 text-center font-medium text-primary sm:px-5">
                                        BetterKV
                                    </th>
                                    <th className="px-4 py-3.5 text-center font-medium text-muted-foreground sm:px-5">
                                        Redis
                                    </th>
                                    <th className="px-4 py-3.5 text-center font-medium text-muted-foreground sm:px-5">
                                        Dragonfly
                                    </th>
                                    <th className="px-4 py-3.5 text-center font-medium text-muted-foreground sm:px-5">
                                        KeyDB
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {features.map((row, i) => (
                                    <tr
                                        key={row.feature}
                                        className={
                                            i % 2 === 0 ? "bg-card/50" : ""
                                        }
                                    >
                                        <td className="px-4 py-3 font-medium sm:px-5">
                                            {row.feature}
                                        </td>
                                        <td className="px-4 py-3 text-center sm:px-5">
                                            <CellValue value={row.betterkv} />
                                        </td>
                                        <td className="px-4 py-3 text-center sm:px-5">
                                            <CellValue value={row.redis} />
                                        </td>
                                        <td className="px-4 py-3 text-center sm:px-5">
                                            <CellValue value={row.dragonfly} />
                                        </td>
                                        <td className="px-4 py-3 text-center sm:px-5">
                                            <CellValue value={row.keydb} />
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </motion.div>
                </div>
            </section>

            <section className="py-24">
                <div className="mx-auto max-w-4xl px-6">
                    <motion.div {...fadeUp}>
                        <h2 className="text-2xl font-bold tracking-tight">
                            Throughput Comparison
                        </h2>
                        <p className="mt-2 text-muted-foreground">
                            GET operations per second, single node, 64 byte
                            values.
                        </p>
                    </motion.div>

                    <div className="mt-10 space-y-4">
                        {throughput.map((item) => (
                            <motion.div
                                key={item.product}
                                {...fadeUp}
                                className="rounded-xl border border-border/50 bg-card p-5"
                            >
                                <div className="mb-3 flex items-center justify-between">
                                    <div className="flex items-center gap-3">
                                        <span
                                            className={`text-sm font-medium ${item.highlight ? "text-primary" : "text-muted-foreground"}`}
                                        >
                                            {item.product}
                                        </span>
                                        {item.highlight && (
                                            <Badge>Fastest</Badge>
                                        )}
                                    </div>
                                    <span className="font-mono text-sm text-muted-foreground">
                                        {item.ops} ops/s
                                    </span>
                                </div>
                                <div className="h-3 overflow-hidden rounded-full bg-muted">
                                    <motion.div
                                        className={`h-full rounded-full ${item.highlight ? "bg-primary" : "bg-muted-foreground/30"}`}
                                        initial={{ width: 0 }}
                                        whileInView={{
                                            width:
                                                item.product === "BetterKV"
                                                    ? "95%"
                                                    : item.product ===
                                                        "Dragonfly"
                                                      ? "42%"
                                                      : item.product === "KeyDB"
                                                        ? "21%"
                                                        : "8%",
                                        }}
                                        viewport={{ once: true }}
                                        transition={{
                                            duration: 1,
                                            ease: "easeOut",
                                        }}
                                    />
                                </div>
                            </motion.div>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}

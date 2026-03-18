import { Link } from "react-router-dom";
import { Logo } from "../Logo";

const footerSections = [
    {
        title: "Product",
        links: [
            { label: "Performance", to: "/performance" },
            { label: "Compatibility", to: "/compatibility" },
            { label: "Security", to: "/security" },
            { label: "Pricing", to: "/pricing" },
            { label: "Roadmap", to: "/roadmap" },
        ],
    },
    {
        title: "Developers",
        links: [
            { label: "Documentation", href: "https://docs.betterkv.com" },
            {
                label: "Installation",
                href: "https://docs.betterkv.com/installation",
            },
            { label: "GitHub", href: "https://github.com/1jmdev/BetterKV" },
            { label: "Changelog", to: "/changelog" },
            { label: "Community", to: "/community" },
            { label: "Compare", to: "/compare" },
        ],
    },
    {
        title: "Use Cases",
        links: [
            { label: "Caching", to: "/use-cases/caching" },
            { label: "Session Storage", to: "/use-cases/sessions" },
            { label: "Analytics", to: "/use-cases/analytics" },
            { label: "Message Queues", to: "/use-cases/queues" },
            { label: "Rate Limiting", to: "/use-cases/rate-limiting" },
            { label: "Feature Flags", to: "/use-cases/feature-flags" },
            { label: "Gaming", to: "/use-cases/gaming" },
        ],
    },
    {
        title: "Company",
        links: [
            { label: "About", to: "/about" },
            { label: "Privacy", to: "/privacy" },
            { label: "Terms", to: "/terms" },
        ],
    },
];

export function Footer() {
    return (
        <footer className="border-t border-border/50 bg-background">
            <div className="mx-auto max-w-6xl px-6 py-16">
                <div className="grid grid-cols-2 gap-8 md:grid-cols-4 lg:grid-cols-5">
                    <div className="col-span-2 md:col-span-4 lg:col-span-1">
                        <Link to="/" className="flex items-center gap-2.5">
                            <Logo />
                            <span className="text-sm font-semibold tracking-tight">
                                BetterKV
                            </span>
                        </Link>
                        <p className="mt-3 max-w-xs text-sm text-muted-foreground">
                            Redis-compatible key-value store built in Rust.
                            5-30x faster.
                        </p>
                    </div>

                    {footerSections.map((section) => (
                        <div key={section.title}>
                            <h4 className="mb-3 text-sm font-medium">
                                {section.title}
                            </h4>
                            <ul className="space-y-2">
                                {section.links.map((link) => (
                                    <li key={link.label}>
                                        {"href" in link && link.href ? (
                                            <a
                                                href={link.href}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className="text-sm text-muted-foreground transition-colors hover:text-foreground"
                                            >
                                                {link.label}
                                            </a>
                                        ) : (
                                            <Link
                                                to={link.to!}
                                                className="text-sm text-muted-foreground transition-colors hover:text-foreground"
                                            >
                                                {link.label}
                                            </Link>
                                        )}
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ))}
                </div>

                <div className="mt-12 flex flex-col items-center justify-between gap-4 border-t border-border/50 pt-8 sm:flex-row">
                    <p className="text-xs text-muted-foreground">
                        &copy; {new Date().getFullYear()} BetterKV. All rights
                        reserved.
                    </p>
                    <div className="flex gap-4">
                        <Link
                            to="/privacy"
                            className="text-xs text-muted-foreground transition-colors hover:text-foreground"
                        >
                            Privacy
                        </Link>
                        <Link
                            to="/terms"
                            className="text-xs text-muted-foreground transition-colors hover:text-foreground"
                        >
                            Terms
                        </Link>
                    </div>
                </div>
            </div>
        </footer>
    );
}

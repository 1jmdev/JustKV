import { useState, useEffect, useRef } from "react";
import { Link } from "react-router-dom";
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
} from "@/components/ui/navigation-menu";
import { Button } from "@/components/ui/button";
import { WaitlistModal } from "@/components/layout/WaitlistModal";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import {
    GaugeIcon,
    ShieldCheckIcon,
    ArrowLeftRightIcon,
    MapIcon,
    TagIcon,
    BookOpenIcon,
    DownloadIcon,
    GitBranchIcon,
    FileTextIcon,
    UsersIcon,
    ScaleIcon,
    LayersIcon,
    DatabaseIcon,
    UserIcon,
    BarChart3Icon,
    MailIcon,
    FlagIcon,
    GamepadIcon,
    ZapIcon,
    MenuIcon,
    ChevronDownIcon,
} from "lucide-react";
import { Logo } from "../Logo";

const productLinks = [
    {
        to: "/performance",
        label: "Performance",
        description: "Benchmarks & architecture",
        icon: GaugeIcon,
    },
    {
        to: "/compatibility",
        label: "Compatibility",
        description: "Redis protocol support",
        icon: ArrowLeftRightIcon,
    },
    {
        to: "/security",
        label: "Security",
        description: "Auth, TLS & encryption",
        icon: ShieldCheckIcon,
    },
    {
        to: "/pricing",
        label: "Pricing",
        description: "Free & cloud tiers",
        icon: TagIcon,
    },
    {
        to: "/roadmap",
        label: "Roadmap",
        description: "What's next",
        icon: MapIcon,
    },
];

const developerLinks = [
    {
        href: "https://docs.betterkv.com",
        label: "Documentation",
        description: "Guides & API reference",
        icon: BookOpenIcon,
        external: true,
    },
    {
        href: "https://docs.betterkv.com/installation",
        label: "Installation",
        description: "Get started in minutes",
        icon: DownloadIcon,
        external: true,
    },
    {
        href: "https://github.com/1jmdev/BetterKV",
        label: "GitHub",
        description: "Source code & issues",
        icon: GitBranchIcon,
        external: true,
    },
    {
        to: "/compare",
        label: "Compare",
        description: "BetterKV vs others",
        icon: ScaleIcon,
    },
    {
        to: "/community",
        label: "Community",
        description: "Get involved",
        icon: UsersIcon,
    },
    {
        to: "/changelog",
        label: "Changelog",
        description: "Release history",
        icon: FileTextIcon,
    },
];

const useCaseLinks = [
    {
        to: "/use-cases",
        label: "Overview",
        description: "All use cases",
        icon: LayersIcon,
    },
    {
        to: "/use-cases/caching",
        label: "Caching",
        description: "Application & API cache",
        icon: ZapIcon,
    },
    {
        to: "/use-cases/sessions",
        label: "Session Storage",
        description: "Fast session management",
        icon: UserIcon,
    },
    {
        to: "/use-cases/analytics",
        label: "Real-time Analytics",
        description: "Counters & leaderboards",
        icon: BarChart3Icon,
    },
    {
        to: "/use-cases/queues",
        label: "Message Queues",
        description: "Pub/Sub & task queues",
        icon: MailIcon,
    },
    {
        to: "/use-cases/rate-limiting",
        label: "Rate Limiting",
        description: "API throttling",
        icon: DatabaseIcon,
    },
    {
        to: "/use-cases/feature-flags",
        label: "Feature Flags",
        description: "Toggle features instantly",
        icon: FlagIcon,
    },
    {
        to: "/use-cases/gaming",
        label: "Gaming",
        description: "Leaderboards & state",
        icon: GamepadIcon,
    },
];

function NavDropdownItem({
    item,
}: {
    item: {
        to?: string;
        href?: string;
        label: string;
        description: string;
        icon: React.ElementType;
        external?: boolean;
    };
}) {
    const Icon = item.icon;

    if (item.external && item.href) {
        return (
            <NavigationMenuLink
                href={item.href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-start gap-3 rounded-md p-2.5 transition-colors hover:bg-accent"
            >
                <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10">
                    <Icon className="size-4 text-primary" />
                </div>
                <div>
                    <div className="text-sm font-medium leading-tight">
                        {item.label}
                    </div>
                    <div className="mt-0.5 text-xs text-muted-foreground">
                        {item.description}
                    </div>
                </div>
            </NavigationMenuLink>
        );
    }

    return (
        <NavigationMenuLink
            render={<Link to={item.to!} />}
            className="flex items-start gap-3 rounded-md p-2.5 transition-colors hover:bg-accent"
        >
            <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10">
                <Icon className="size-4 text-primary" />
            </div>
            <div>
                <div className="text-sm font-medium leading-tight">
                    {item.label}
                </div>
                <div className="mt-0.5 text-xs text-muted-foreground">
                    {item.description}
                </div>
            </div>
        </NavigationMenuLink>
    );
}

function MobileNavSection({
    title,
    links,
    onClose,
}: {
    title: string;
    links: Array<{
        to?: string;
        href?: string;
        label: string;
        icon: React.ElementType;
        external?: boolean;
    }>;
    onClose: () => void;
}) {
    const [open, setOpen] = useState(false);

    return (
        <div>
            <button
                onClick={() => setOpen(!open)}
                className="flex w-full items-center justify-between py-2 text-sm font-medium"
            >
                {title}
                <ChevronDownIcon
                    className={`size-4 transition-transform ${open ? "rotate-180" : ""}`}
                />
            </button>
            {open && (
                <div className="mt-1 ml-3 space-y-1 border-l border-border/50 pl-3">
                    {links.map((link) => {
                        const Icon = link.icon;
                        if (link.external && link.href) {
                            return (
                                <a
                                    key={link.label}
                                    href={link.href}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    onClick={onClose}
                                    className="flex items-center gap-2 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
                                >
                                    <Icon className="size-3.5 shrink-0" />
                                    {link.label}
                                </a>
                            );
                        }
                        return (
                            <Link
                                key={link.label}
                                to={link.to!}
                                onClick={onClose}
                                className="flex items-center gap-2 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
                            >
                                <Icon className="size-3.5 shrink-0" />
                                {link.label}
                            </Link>
                        );
                    })}
                </div>
            )}
        </div>
    );
}

export function Navbar() {
    const [mobileOpen, setMobileOpen] = useState(false);
    const [openMenu, setOpenMenu] = useState<string | false>(false);
    const scrollY = useRef(0);

    // Close dropdowns on scroll
    useEffect(() => {
        function handleScroll() {
            const currentY = window.scrollY;
            if (Math.abs(currentY - scrollY.current) > 5) {
                setOpenMenu(false);
            }
            scrollY.current = currentY;
        }
        window.addEventListener("scroll", handleScroll, { passive: true });
        return () => window.removeEventListener("scroll", handleScroll);
    }, []);

    return (
        <header className="sticky top-0 z-50 w-full border-b border-border/50 bg-background/80 backdrop-blur-xl">
            <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-4 sm:px-6">
                {/* Logo */}
                <div className="flex items-center gap-1">
                    <Link to="/" className="mr-4 flex items-center gap-2.5">
                        <Logo />
                        <span className="text-sm font-semibold tracking-tight">
                            BetterKV
                        </span>
                    </Link>

                    {/* Desktop Navigation */}
                    <NavigationMenu
                        className="hidden md:flex"
                        value={openMenu}
                        onValueChange={setOpenMenu}
                    >
                        <NavigationMenuList>
                            <NavigationMenuItem>
                                <NavigationMenuTrigger>
                                    Product
                                </NavigationMenuTrigger>
                                <NavigationMenuContent className="w-85">
                                    <div className="grid gap-0.5 p-1.5">
                                        {productLinks.map((item) => (
                                            <NavDropdownItem
                                                key={item.label}
                                                item={item}
                                            />
                                        ))}
                                    </div>
                                </NavigationMenuContent>
                            </NavigationMenuItem>

                            <NavigationMenuItem>
                                <NavigationMenuTrigger>
                                    Developers
                                </NavigationMenuTrigger>
                                <NavigationMenuContent className="w-85">
                                    <div className="grid gap-0.5 p-1.5">
                                        {developerLinks.map((item) => (
                                            <NavDropdownItem
                                                key={item.label}
                                                item={item}
                                            />
                                        ))}
                                    </div>
                                </NavigationMenuContent>
                            </NavigationMenuItem>

                            <NavigationMenuItem>
                                <NavigationMenuTrigger>
                                    Use Cases
                                </NavigationMenuTrigger>
                                <NavigationMenuContent className="w-95">
                                    <div className="grid grid-cols-2 gap-0.5 p-1.5">
                                        {useCaseLinks.map((item) => (
                                            <NavigationMenuLink
                                                key={item.label}
                                                render={<Link to={item.to} />}
                                                className="flex items-start gap-2.5 rounded-md p-2.5 transition-colors hover:bg-accent"
                                            >
                                                <item.icon className="mt-0.5 size-4 shrink-0 text-primary" />
                                                <div>
                                                    <div className="text-sm font-medium leading-tight">
                                                        {item.label}
                                                    </div>
                                                    <div className="mt-0.5 text-xs text-muted-foreground">
                                                        {item.description}
                                                    </div>
                                                </div>
                                            </NavigationMenuLink>
                                        ))}
                                    </div>
                                </NavigationMenuContent>
                            </NavigationMenuItem>
                        </NavigationMenuList>
                    </NavigationMenu>
                </div>

                {/* Right side */}
                <div className="flex items-center gap-2">
                    {/* GitHub icon - hidden on smallest screens */}
                    <Button
                        variant="ghost"
                        size="icon"
                        className="hidden sm:inline-flex"
                        render={
                            <a
                                href="https://github.com/1jmdev/BetterKV"
                                target="_blank"
                                rel="noopener noreferrer"
                                aria-label="GitHub"
                            />
                        }
                    >
                        <svg
                            viewBox="0 0 24 24"
                            className="size-4"
                            fill="currentColor"
                        >
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                        </svg>
                    </Button>
                    <WaitlistModal>
                        <Button
                            size="sm"
                            className="hidden sm:inline-flex cursor-pointer"
                        >
                            Join Waitlist
                        </Button>
                    </WaitlistModal>

                    {/* Mobile hamburger */}
                    <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
                        <SheetTrigger
                            render={
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    className="md:hidden"
                                    aria-label="Open menu"
                                />
                            }
                        >
                            <MenuIcon className="size-5" />
                        </SheetTrigger>
                        <SheetContent
                            side="right"
                            className="w-70 overflow-y-auto p-0"
                            showCloseButton={false}
                        >
                            {/* Header with logo + close */}
                            <div className="flex h-14 items-center justify-between border-b border-border/50 px-4">
                                <Link
                                    to="/"
                                    onClick={() => setMobileOpen(false)}
                                    className="flex items-center gap-2.5"
                                >
                                    <Logo />
                                    <span className="text-sm font-semibold tracking-tight">
                                        BetterKV
                                    </span>
                                </Link>
                                <button
                                    onClick={() => setMobileOpen(false)}
                                    className="flex size-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:text-foreground"
                                    aria-label="Close menu"
                                >
                                    <svg
                                        viewBox="0 0 24 24"
                                        className="size-4"
                                        fill="none"
                                        stroke="currentColor"
                                        strokeWidth="2"
                                    >
                                        <path d="M18 6 6 18M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>

                            {/* Nav sections */}
                            <div className="flex flex-col gap-0.5 px-4 py-4">
                                <MobileNavSection
                                    title="Product"
                                    links={productLinks}
                                    onClose={() => setMobileOpen(false)}
                                />
                                <MobileNavSection
                                    title="Developers"
                                    links={developerLinks}
                                    onClose={() => setMobileOpen(false)}
                                />
                                <MobileNavSection
                                    title="Use Cases"
                                    links={useCaseLinks}
                                    onClose={() => setMobileOpen(false)}
                                />

                                <div className="mt-4 flex flex-col gap-2 border-t border-border/50 pt-4">
                                    <a
                                        href="https://github.com/1jmdev/BetterKV"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
                                        onClick={() => setMobileOpen(false)}
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            className="size-4"
                                            fill="currentColor"
                                        >
                                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                                        </svg>
                                        GitHub
                                    </a>
                                    <WaitlistModal>
                                        <Button
                                            size="sm"
                                            className="w-full cursor-pointer"
                                        >
                                            Join Waitlist
                                        </Button>
                                    </WaitlistModal>
                                </div>
                            </div>
                        </SheetContent>
                    </Sheet>
                </div>
            </div>
        </header>
    );
}

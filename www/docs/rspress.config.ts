import * as path from "node:path";
import { defineConfig } from "@rspress/core";
import { pluginSitemap } from "@rspress/plugin-sitemap";

export default defineConfig({
    root: path.join(import.meta.dirname, "pages"),
    plugins: [
        pluginSitemap({
            siteUrl: "https://docs.betterkv.com",
        }),
    ],
    title: "BetterKV Docs",
    description:
        "The high-performance Redis-compatible key-value store. Lightning-fast, reliable, open-source.",
    icon: "/icon.svg",
    logo: {
        light: "/logo-light.svg",
        dark: "/logo-dark.svg",
    },
    globalStyles: path.join(import.meta.dirname, "styles", "global.css"),
    themeConfig: {
        footer: {
            message:
                "© 2026 BetterKV — Open-source, high-performance key-value store built in rust.",
        },
        socialLinks: [
            {
                icon: "github",
                mode: "link",
                content: "https://github.com/1jmdev/BetterKV",
            },
        ],
        nav: [
            {
                text: "Docs",
                link: "/",
                activeMatch:
                    "^/$|^/(quick-start|installation|configuration|data-types|persistence|replication|cluster|lua-scripting|pubsub|transactions|security)",
            },
            { text: "Commands", link: "/commands/", activeMatch: "/commands/" },
            { text: "API", link: "/api/", activeMatch: "/api/" },
        ],
        sidebar: {
            "/": [
                {
                    text: "Getting Started",
                    items: [
                        { text: "Introduction", link: "/" },
                        { text: "Quick Start", link: "/quick-start" },
                        { text: "Installation", link: "/installation" },
                        { text: "Configuration", link: "/configuration" },
                    ],
                },
                {
                    text: "Core Concepts",
                    items: [
                        { text: "Data Types", link: "/data-types" },
                        { text: "Persistence", link: "/persistence" },
                        { text: "Replication", link: "/replication" },
                        { text: "Cluster Mode", link: "/cluster" },
                    ],
                },
                {
                    text: "Advanced",
                    items: [
                        { text: "Lua Scripting", link: "/lua-scripting" },
                        { text: "Pub/Sub", link: "/pubsub" },
                        { text: "Transactions", link: "/transactions" },
                        { text: "Security", link: "/security" },
                    ],
                },
            ],
            "/commands/": [
                {
                    text: "Command Reference",
                    items: [
                        { text: "Overview", link: "/commands/" },
                        { text: "Connection & Server", link: "/commands/server" },
                        { text: "Keys & Expiry", link: "/commands/keys" },
                        { text: "Scripting", link: "/commands/scripting" },
                    ],
                },
                {
                    text: "String & Numeric",
                    items: [
                        { text: "Strings", link: "/commands/string" },
                        { text: "Numeric", link: "/commands/numeric" },
                    ],
                },
                {
                    text: "Data Structures",
                    items: [
                        { text: "Lists", link: "/commands/list" },
                        { text: "Hashes", link: "/commands/hash" },
                        { text: "Sets", link: "/commands/set" },
                        { text: "Sorted Sets", link: "/commands/sorted-set" },
                        { text: "GEO", link: "/commands/geo" },
                        { text: "Streams", link: "/commands/stream" },
                    ],
                },
            ],
            "/api/": [
                {
                    text: "REST API",
                    items: [
                        { text: "Overview", link: "/api/" },
                        { text: "Authentication", link: "/api/auth" },
                        { text: "Endpoints", link: "/api/endpoints" },
                    ],
                },
            ],
        },
        editLink: {
            docRepoBaseUrl:
                "https://github.com/1jmdev/BetterKV/tree/main/www/docs/pages",
        },
        lastUpdated: true,
    },
    markdown: {
        showLineNumbers: true,
    },
});

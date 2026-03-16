import { Hono } from "hono";
import { D1Database, RateLimit } from "@cloudflare/workers-types";

type CloudflareBindings = {
    RATELIMIT: RateLimit;
    betterkv_waitlist: D1Database;
};

type WaitlistRequest = {
    email?: string;
};

const emailPattern = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;

const app = new Hono<{ Bindings: CloudflareBindings }>();

app.get("/", (c) => {
    return c.json({ ok: true });
});

app.post("/waitlist", async (c) => {
    const clientAddress = c.req.header("CF-Connecting-IP") ?? "unknown";
    const rateLimit = await c.env.RATELIMIT.limit({ key: clientAddress });

    if (!rateLimit.success) {
        return c.json({ error: "Too many requests" }, 429);
    }

    const body = await c.req.json<WaitlistRequest>().catch(() => null);
    const normalizedEmail = body?.email?.trim().toLowerCase();

    if (!normalizedEmail || !emailPattern.test(normalizedEmail)) {
        return c.json({ error: "Invalid email" }, 400);
    }

    const existingEntry = await c.env.betterkv_waitlist
        .prepare("SELECT email FROM waitlist WHERE email = ? LIMIT 1")
        .bind(normalizedEmail)
        .first<{ email: string }>();

    if (existingEntry) {
        return c.json({ ok: true, added: false });
    }

    await c.env.betterkv_waitlist
        .prepare("INSERT INTO waitlist (email) VALUES (?)")
        .bind(normalizedEmail)
        .run();

    return c.json({ ok: true, added: true }, 201);
});

export default app;

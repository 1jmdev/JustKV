import { useState } from "react";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CheckIcon } from "lucide-react";

export function WaitlistModal({ children }: { children: React.ReactNode }) {
    const [email, setEmail] = useState("");
    const [submitted, setSubmitted] = useState(false);
    const [open, setOpen] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        if (!email || isSubmitting) return;

        setIsSubmitting(true);
        setError(null);

        try {
            const response = await fetch("https://api.betterkv.com/waitlist", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({ email }),
            });

            if (!response.ok) {
                const payload = (await response.json().catch(() => null)) as
                    | { error?: string }
                    | null;

                throw new Error(payload?.error ?? "Failed to join waitlist");
            }

            setSubmitted(true);
            setEmail("");
        } catch (submissionError) {
            setError(
                submissionError instanceof Error
                    ? submissionError.message
                    : "Failed to join waitlist",
            );
        } finally {
            setIsSubmitting(false);
        }
    }

    return (
        <Dialog
            open={open}
            onOpenChange={(nextOpen) => {
                setOpen(nextOpen);
                if (!nextOpen) {
                    setTimeout(() => {
                        setSubmitted(false);
                        setError(null);
                    }, 300);
                }
            }}
        >
            <DialogTrigger
                onClick={() => setOpen(true)}
                render={<span className="inline-flex" />}
            >
                {children}
            </DialogTrigger>
            <DialogContent className="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>
                        {submitted ? "You're on the list" : "Join the Waitlist"}
                    </DialogTitle>
                    <DialogDescription>
                        {submitted
                            ? "We'll notify you when BetterKV Cloud is ready."
                            : "Get early access to BetterKV Cloud — managed, hosted, and ready to scale."}
                    </DialogDescription>
                </DialogHeader>
                {submitted ? (
                    <div className="flex items-center gap-3 rounded-lg bg-primary/10 p-4">
                        <div className="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary/20">
                            <CheckIcon className="size-4 text-primary" />
                        </div>
                        <p className="text-sm text-muted-foreground">
                            We'll send updates to your inbox.
                        </p>
                    </div>
                ) : (
                    <div className="space-y-3">
                        <form onSubmit={handleSubmit} className="flex gap-2">
                            <Input
                                type="email"
                                placeholder="you@example.com"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                required
                                className="flex-1"
                            />
                            <Button type="submit" disabled={isSubmitting}>
                                {isSubmitting ? "Submitting..." : "Subscribe"}
                            </Button>
                        </form>
                        {error ? (
                            <p className="text-sm text-destructive">{error}</p>
                        ) : null}
                    </div>
                )}
            </DialogContent>
        </Dialog>
    );
}

import { PageHeader } from "@/components/layout/PageHeader";
import { Seo } from "@/components/Seo";

export function PrivacyPage() {
    return (
        <div>
            <Seo
                title="Privacy Policy — BetterKV"
                description="BetterKV's privacy policy. How we collect, use, and protect your data."
                path="/privacy"
                noindex={true}
            />
            <PageHeader
                title="Privacy Policy"
                description="How we handle your data. Last updated March 2026."
            />

            <section className="py-24">
                <div className="mx-auto max-w-3xl px-6 space-y-10">
                    <div>
                        <h2 className="text-lg font-semibold">
                            1. Information We Collect
                        </h2>
                        <div className="mt-3 space-y-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                When you join our waitlist, we collect your
                                email address. When you visit our website, we
                                may collect standard web analytics data
                                including page views, referrer information, and
                                browser type.
                            </p>
                            <p>
                                BetterKV the software does not collect any
                                telemetry or usage data. What runs on your
                                servers stays on your servers.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            2. How We Use Your Information
                        </h2>
                        <div className="mt-3 space-y-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                Email addresses collected through our waitlist
                                are used solely to notify you about BetterKV
                                Cloud availability and major product updates. We
                                will never sell your email address to third
                                parties.
                            </p>
                            <p>
                                Web analytics data is used to understand how
                                visitors use our website and to improve the user
                                experience.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            3. Data Storage
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                Your data is stored securely and encrypted at
                                rest. We use industry-standard security
                                practices to protect your information.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">4. Cookies</h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                We use minimal, essential cookies for basic
                                website functionality. We do not use tracking
                                cookies or third-party advertising cookies.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            5. Third-Party Services
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                We may use third-party services for web
                                analytics and email delivery. These services are
                                bound by their own privacy policies and our data
                                processing agreements.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            6. Your Rights
                        </h2>
                        <div className="mt-3 space-y-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                You have the right to access, correct, or delete
                                your personal data at any time. To exercise
                                these rights, contact us at{" "}
                                <span className="font-mono text-primary">
                                    privacy@betterkv.com
                                </span>
                                .
                            </p>
                            <p>
                                You can unsubscribe from our mailing list at any
                                time using the link in any email we send you.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            7. Changes to This Policy
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                We may update this privacy policy from time to
                                time. Any changes will be posted on this page
                                with an updated revision date.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">8. Contact</h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                For any questions about this privacy policy,
                                contact us at{" "}
                                <span className="font-mono text-primary">
                                    privacy@betterkv.com
                                </span>
                                .
                            </p>
                        </div>
                    </div>
                </div>
            </section>
        </div>
    );
}

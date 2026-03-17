import { PageHeader } from "@/components/layout/PageHeader";
import { Seo } from "@/components/Seo";

export function TermsPage() {
    return (
        <div>
            <Seo
                title="Terms of Service — BetterKV"
                description="BetterKV's terms of service. The terms governing your use of BetterKV software and services."
                path="/terms"
                noindex={true}
            />
            <PageHeader
                title="Terms of Service"
                description="Terms governing your use of BetterKV. Last updated March 2026."
            />

            <section className="py-24">
                <div className="mx-auto max-w-3xl px-6 space-y-10">
                    <div>
                        <h2 className="text-lg font-semibold">
                            1. Acceptance of Terms
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                By accessing or using BetterKV software,
                                website, or services, you agree to be bound by
                                these terms. If you do not agree, do not use our
                                services.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">2. License</h2>
                        <div className="mt-3 space-y-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                BetterKV is open source software. The source
                                code is licensed under the terms specified in
                                the LICENSE file in our GitHub repository. You
                                are free to use, modify, and distribute BetterKV
                                in accordance with that license.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            3. Website Use
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                You may use our website for lawful purposes
                                only. You agree not to use our website in any
                                way that could damage, disable, or impair the
                                site or interfere with other users.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            4. Waitlist & Communications
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                By joining our waitlist, you consent to receive
                                email communications about BetterKV Cloud and
                                major product updates. You can unsubscribe at
                                any time.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            5. Disclaimer of Warranties
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                BetterKV is provided "as is" without warranty of
                                any kind, express or implied. We do not
                                guarantee that the software will be error-free
                                or that it will meet your specific requirements.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            6. Limitation of Liability
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                In no event shall BetterKV or its contributors
                                be liable for any indirect, incidental, special,
                                or consequential damages arising out of or in
                                connection with the use of the software or
                                services.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            7. BetterKV Cloud (Future)
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                BetterKV Cloud, when available, will be governed
                                by a separate service agreement. Additional
                                terms will be provided at that time.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">
                            8. Changes to Terms
                        </h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                We reserve the right to update these terms at
                                any time. Continued use of our services after
                                changes are posted constitutes acceptance of the
                                new terms.
                            </p>
                        </div>
                    </div>

                    <div>
                        <h2 className="text-lg font-semibold">9. Contact</h2>
                        <div className="mt-3 text-sm text-muted-foreground leading-relaxed">
                            <p>
                                For questions about these terms, contact us at{" "}
                                <span className="font-mono text-primary">
                                    legal@betterkv.com
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

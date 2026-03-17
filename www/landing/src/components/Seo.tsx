import { Helmet } from "react-helmet-async";

const SITE_URL = "https://betterkv.com";
const DEFAULT_IMAGE = `${SITE_URL}/og-image.png`;
const TWITTER_HANDLE = "@betterkv";

interface SeoProps {
    title: string;
    description: string;
    path?: string;
    image?: string;
    type?: "website" | "article";
    noindex?: boolean;
    jsonLd?: object;
}

export function Seo({
    title,
    description,
    path = "/",
    image = DEFAULT_IMAGE,
    type = "website",
    noindex = false,
    jsonLd,
}: SeoProps) {
    const canonical = `${SITE_URL}${path}`;
    const fullTitle = title.includes("BetterKV")
        ? title
        : `${title} | BetterKV`;

    return (
        <Helmet>
            <title>{fullTitle}</title>
            <meta name="description" content={description} />
            <link rel="canonical" href={canonical} />
            {noindex && <meta name="robots" content="noindex,nofollow" />}

            {/* Open Graph */}
            <meta property="og:title" content={fullTitle} />
            <meta property="og:description" content={description} />
            <meta property="og:url" content={canonical} />
            <meta property="og:type" content={type} />
            <meta property="og:image" content={image} />
            <meta property="og:image:width" content="1200" />
            <meta property="og:image:height" content="630" />
            <meta property="og:image:alt" content={fullTitle} />
            <meta property="og:site_name" content="BetterKV" />
            <meta property="og:locale" content="en_US" />

            {/* Twitter Card */}
            <meta name="twitter:card" content="summary_large_image" />
            <meta name="twitter:site" content={TWITTER_HANDLE} />
            <meta name="twitter:creator" content={TWITTER_HANDLE} />
            <meta name="twitter:title" content={fullTitle} />
            <meta name="twitter:description" content={description} />
            <meta name="twitter:image" content={image} />
            <meta name="twitter:image:alt" content={fullTitle} />

            {/* JSON-LD structured data */}
            {jsonLd && (
                <script type="application/ld+json">
                    {JSON.stringify(jsonLd)}
                </script>
            )}
        </Helmet>
    );
}

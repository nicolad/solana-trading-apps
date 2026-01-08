export default {
    async fetch(request, env) {
        const url = new URL(request.url);
        if (url.pathname === "/") {
            return new Response([
                "LaserStream Dev Server (No Container)",
                "",
                "This is a simplified version for local development.",
                "For full Container deployment, use: pnpm dev:full",
                "",
                "Environment:",
                `- LASERSTREAM_ENDPOINT: ${env.LASERSTREAM_ENDPOINT || "not set"}`,
                `- HELIUS_API_KEY: ${env.HELIUS_API_KEY
                    ? "***" + env.HELIUS_API_KEY.slice(-4)
                    : "not set"}`,
                "",
                "Note: Full LaserStream functionality requires Cloudflare Containers.",
                "Run the standalone Rust service instead:",
                "  cd ../.. && cargo run",
            ].join("\n"), { headers: { "content-type": "text/plain; charset=utf-8" } });
        }
        return new Response("Not found", { status: 404 });
    },
};

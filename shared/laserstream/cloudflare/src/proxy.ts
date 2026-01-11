// Simple Cloudflare Worker that proxies to external LaserStream service
// No containers, no Docker, just a lightweight proxy

export interface Env {
  LASERSTREAM_SERVICE_URL: string; // External service URL
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // Health check
    if (url.pathname === "/") {
      return new Response(
        [
          "LaserStream Proxy",
          "",
          `Proxying to: ${env.LASERSTREAM_SERVICE_URL}`,
          "",
          "Endpoints:",
          "GET  /health  - Service health",
          "POST /start   - Start LaserStream",
          "GET  /latest  - Latest slot update",
        ].join("\n"),
        { headers: { "content-type": "text/plain" } }
      );
    }

    // Proxy all other requests to the external LaserStream service
    const proxyUrl = new URL(
      url.pathname + url.search,
      env.LASERSTREAM_SERVICE_URL
    );

    const proxyRequest = new Request(proxyUrl.toString(), {
      method: request.method,
      headers: request.headers,
      body: request.body,
    });

    try {
      return await fetch(proxyRequest);
    } catch (error) {
      return new Response(
        JSON.stringify({
          error: "Failed to connect to LaserStream service",
          message: error instanceof Error ? error.message : "Unknown error",
          service: env.LASERSTREAM_SERVICE_URL,
        }),
        {
          status: 502,
          headers: { "content-type": "application/json" },
        }
      );
    }
  },
};

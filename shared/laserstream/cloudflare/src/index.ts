import { Container, getContainer } from "@cloudflare/containers";

export interface Env {
  LASER_CONTAINER: any; // Container namespace
  HELIUS_API_KEY: string; // secret
  LASERSTREAM_ENDPOINT: string; // var
}

export class LaserStreamContainer extends Container {
  // Your Rust service listens on 8080 inside the container
  defaultPort = 8080;
}

function help(origin: string) {
  return [
    "LaserStream (Rust SDK) on Cloudflare Containers",
    "",
    `GET  ${origin}/health`,
    `POST ${origin}/start`,
    `GET  ${origin}/latest`,
    "",
    "Notes:",
    "- /start is idempotent; it starts the LaserStream background task inside the container.",
    "- /latest returns the last slot update seen (JSON).",
  ].join("\n");
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === "/") {
      return new Response(help(url.origin), {
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }

    // One singleton container instance by name
    const id = env.LASER_CONTAINER.idFromName("main");
    const container = getContainer(env.LASER_CONTAINER, id);

    // Set environment variables for the container
    await container.setEnvironmentVariables({
      HELIUS_API_KEY: env.HELIUS_API_KEY,
      LASERSTREAM_ENDPOINT: env.LASERSTREAM_ENDPOINT,
      RUST_LOG: "info",
    });

    // Proxy the request path to the container.
    // The container only cares about path/query/method/headers/body.
    const forwardUrl = new URL(request.url);
    forwardUrl.protocol = "http:";
    forwardUrl.hostname = "container";
    forwardUrl.port = "8080";

    const proxied = new Request(forwardUrl.toString(), request);
    return container.fetch(proxied);
  },
};

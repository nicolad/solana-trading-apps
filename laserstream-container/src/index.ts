import { Container } from "@cloudflare/containers";
import { Hono } from "hono";

export class LaserStreamContainer extends Container<Env> {
  defaultPort = 8080;
  sleepAfter = "2m";

  // Environment variables for the container
  // Note: HELIUS_API_KEY must be set via wrangler secret and passed from the Worker
  override get envVars() {
    return {
      HELIUS_API_KEY: this.env.HELIUS_API_KEY || "",
      LASERSTREAM_ENDPOINT: "https://laserstream-devnet-ewr.helius-rpc.com",
      RUST_LOG: "debug",
    };
  }

  override onStart() {
    console.log("LaserStream container started");
    console.log("HELIUS_API_KEY configured:", !!this.env.HELIUS_API_KEY);
  }

  override onStop() {
    console.log("LaserStream container stopped");
  }

  override onError(error: unknown) {
    console.log("LaserStream container error:", error);
  }
}

const app = new Hono<{ Bindings: Env }>();

app.get("/", (c) => {
  return c.text(
    "LaserStream on Cloudflare Containers\n\n" +
      "Endpoints:\n" +
      "GET  /health - Health check\n" +
      "POST /start - Start LaserStream subscription\n" +
      "GET  /latest - Get latest slot update\n"
  );
});

app.get("/health", (c) => {
  return c.json({ status: "ok", timestamp: new Date().toISOString() });
});

// Proxy all other requests to the singleton LaserStream container
app.all("*", async (c) => {
  try {
    const containerId =
      c.env.LASERSTREAM_CONTAINER.idFromName("laserstream-main");
    const container = c.env.LASERSTREAM_CONTAINER.get(containerId);
    return await container.fetch(c.req.raw);
  } catch (error) {
    console.error("Container error:", error);
    return c.json(
      { error: "Container unavailable", details: String(error) },
      500
    );
  }
});

export default app;

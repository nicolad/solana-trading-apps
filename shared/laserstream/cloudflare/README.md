# LaserStream Rust SDK on Cloudflare (Containers)

This is the smallest working setup to run the **Helius LaserStream Rust client** on Cloudflare by using **Cloudflare Containers** (Linux runtime), with a tiny Worker proxy.

## What you get

- `POST /start` starts the LaserStream slot subscription inside the container (idempotent).
- `GET /latest` returns the most recent slot update as JSON.
- `GET /health` sanity check.

## Requirements

- Cloudflare Workers **Paid plan** with **Containers** enabled (Containers aren't $0).  
- A Helius plan that includes LaserStream access (devnet and mainnet have plan requirements).

## Setup

1. **Install dependencies**

```bash
npm i
```

1. **Set Worker secret for your Helius key (x-token)**

```bash
wrangler secret put HELIUS_API_KEY
```

1. **Set your LaserStream endpoint in `wrangler.toml`**:

```toml
[vars]
LASERSTREAM_ENDPOINT = "https://laserstream-mainnet-<region>.helius-rpc.com"
```

1. **Deploy**

```bash
npm run deploy
```

## Test

```bash
# Start stream (idempotent)
curl -X POST https://<your-worker>.workers.dev/start

# Get latest slot
curl https://<your-worker>.workers.dev/latest
```

If `/latest` returns `no data yet`, wait a moment and retry.

## Development vs Production

### Local Development (`pnpm dev`)

- Uses simplified TypeScript worker (no Rust container)
- Config: `wrangler.dev.toml`
- Entry: `src/index.dev.ts`
- **No Docker required**
- Shows environment info and guides to standalone Rust service

### Production Deployment (`pnpm deploy`)

- Uses full Rust container with LaserStream SDK
- Config: `wrangler.toml`
- Entry: `src/index.ts` + `container/`
- **Docker runs on Cloudflare's build servers** (not your local machine)
- Full gRPC streaming functionality

## Why Containers (Production Only)?

- LaserStream Rust SDK requires gRPC + Tokio runtime
- Standard Workers don't support full Rust networking stacks
- Cloudflare Containers provide Linux environment
- Docker build happens on Cloudflare's infrastructure during deployment

See [CLOUDFLARE_DEPLOYMENT.md](../CLOUDFLARE_DEPLOYMENT.md) for full details.

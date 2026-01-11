# LaserStream Container

Real-time Solana slot updates using Helius LaserStream SDK running in a Cloudflare Container.

## Architecture

- **Worker (TypeScript)**: Hono-based HTTP API and routing layer
- **Container (Rust)**: Axum HTTP server running the LaserStream gRPC client
- **Durable Object**: SQLite-backed singleton managing container lifecycle

## Deployment

Live at: **https://laserstream-container.eeeew.workers.dev/**

### Container Image

- **Registry**: `registry.cloudflare.com/a036f50e02431c89170b8f977e982a3d/laserstream-container-rust`
- **Tag**: `v1.0.0`
- **Digest**: `sha256:59c03a69b0570010635fa9c9ef620c1c888b9fb19b61f09ff479ef8bf0b01c2f`

## API Endpoints

### Worker Endpoints
- `GET /` - Service information
- `GET /health` - Worker health check

### Container Endpoints (proxied through Worker)
- `POST /start` - Start LaserStream subscription
- `GET /latest` - Get latest slot update

## Development

### Prerequisites

- Docker Desktop (for building containers locally)
- Rust 1.83+ (optional, for local Rust development)
- Node.js 20+
- pnpm

### Build Container

```bash
# Build the Rust container image locally
pnpm run build:container

# Tag with version
docker tag laserstream-container-rust:latest laserstream-container-rust:v1.0.1

# Push to Cloudflare registry
docker tag laserstream-container-rust:latest laserstream-container-rust:v1.0.1
pnpm run push:container
```

### Deploy

```bash
# Deploy Worker + Container to Cloudflare
pnpm run deploy
```

### Set Secrets

```bash
# Set Helius API key (required for LaserStream)
echo "YOUR_HELIUS_API_KEY" | pnpm run secret:set HELIUS_API_KEY
```

### Local Development

```bash
# Start local development server (Worker only, no container locally)
pnpm run dev
```

**Note**: The LaserStream Rust SDK cannot run inside Cloudflare Workers due to gRPC requirements. It must run in a Container, which is only available on Cloudflare infrastructure (not locally).

## Configuration

### Environment Variables

Set in `wrangler.jsonc`:

- `LASERSTREAM_ENDPOINT`: Helius LaserStream gRPC endpoint (default: devnet)

### Secrets

Set via `wrangler secret put`:

- `HELIUS_API_KEY`: Your Helius API key (required)

### Container Settings

- **Max Instances**: 10
- **Default Port**: 8080
- **Sleep After**: 2 minutes of inactivity
- **Instance Type**: lite (256 MiB RAM, 1/16 vCPU)

## Rust Container Source

Located in `container_src/`:

- `src/main.rs` - Axum HTTP server
- `src/stream.rs` - LaserStream gRPC client
- `Cargo.toml` - Rust dependencies

### Key Dependencies

- `axum` 0.7 - HTTP server
- `helius-laserstream` 0.1.5 - LaserStream SDK
- `tokio` 1.49 - Async runtime
- `tracing` - Logging

## Testing

```bash
# Health check (Worker)
curl https://laserstream-container.eeeew.workers.dev/health

# Start LaserStream subscription
curl -X POST https://laserstream-container.eeeew.workers.dev/start

# Get latest slot update
curl https://laserstream-container.eeeew.workers.dev/latest
```

## Notes

- Container instances spin up on-demand (first request takes ~10-30s)
- Container sleeps after 2 minutes of inactivity to save resources
- LaserStream connection requires valid Helius API key
- Currently configured for Solana devnet

## Resources

- [Cloudflare Containers Docs](https://developers.cloudflare.com/containers/)
- [Helius LaserStream](https://docs.helius.dev/laserstream/overview)
- [Helius LaserStream SDK](https://github.com/helius-labs/helius-laserstream-rust)

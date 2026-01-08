# Remote Build Strategy for Cloudflare Containers

## Problem
Cloudflare Containers currently require Docker CLI locally to build container images before deployment. This conflicts with our "Docker only on Cloudflare" approach.

## Solution: CI/CD Remote Builds

Since Cloudflare doesn't yet support building containers entirely on their infrastructure during `wrangler deploy`, we use GitHub Actions as a remote build environment.

### How It Works

1. **Local Development**: Use simplified TypeScript worker (no containers)
   ```bash
   pnpm laserstream:dev
   ```

2. **Production Deployment**: Push to GitHub, CI/CD builds & deploys
   ```bash
   git add .
   git commit -m "Update LaserStream"
   git push origin main
   ```

3. **GitHub Actions** (runs on Ubuntu with Docker pre-installed):
   - Checks out code
   - Installs pnpm dependencies
   - Builds TypeScript
   - Builds Docker container using Docker daemon on GitHub's servers
   - Deploys to Cloudflare using wrangler

### Setup Instructions

1. **Get Cloudflare API Token**:
   - Go to https://dash.cloudflare.com/profile/api-tokens
   - Create Token → Edit Cloudflare Workers
   - Permissions: Account.Workers Scripts (Edit)
   - Copy the token

2. **Add to GitHub Secrets**:
   - Go to your repo → Settings → Secrets and variables → Actions
   - New repository secret
   - Name: `CLOUDFLARE_API_TOKEN`
   - Value: [paste your token]

3. **Deploy**:
   ```bash
   git push origin main
   ```
   - GitHub Actions will automatically build and deploy
   - Docker runs on GitHub's infrastructure, not locally
   - View progress: Actions tab in GitHub

### Manual Deployment (Alternative)

If you need to deploy manually without GitHub Actions:

**Option A: Use a Cloud Build Service**
```bash
# Push to container registry first
docker buildx create --use --driver cloud <org>/<builder>
docker buildx build --platform linux/amd64 -t ghcr.io/username/laserstream:latest --push ./container

# Update wrangler.toml to use pre-built image
# [[containers]]
# class_name = "LaserStreamContainer"
# image = "ghcr.io/username/laserstream:latest"

pnpm deploy
```

**Option B: Temporarily Install Docker**
```bash
brew install --cask docker
# Open Docker Desktop
pnpm laserstream:deploy
# Uninstall Docker after deployment
```

### Why This Approach?

- ✅ No Docker needed on developer machines
- ✅ Consistent builds (Ubuntu on GitHub)
- ✅ CI/CD best practices
- ✅ Build logs and artifacts tracked
- ✅ Easy rollbacks via GitHub
- ✅ Works with team (everyone gets same build environment)

### Future: True Remote Builds

Cloudflare may add support for building containers entirely on their infrastructure in future wrangler versions. Watch for:
- `wrangler deploy --remote-build` flag
- Container registry integration
- Build service API

Until then, GitHub Actions provides a practical remote build solution.

## Workflow File

See [.github/workflows/deploy-laserstream.yml](../../../.github/workflows/deploy-laserstream.yml) for the complete GitHub Actions workflow.

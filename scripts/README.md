# scripts/

Utility scripts for managing the monorepo.

## Managing Secrets

### Set Cloudflare Secrets

Use wrangler directly to manage secrets:

```bash
# From repository root
pnpm run container:secret HELIUS_API_KEY
# Then paste your API key when prompted

# Or pipe the value
echo "your_api_key" | pnpm run container:secret HELIUS_API_KEY
```

### Get a Helius API Key

1. Visit [https://dev.helius.xyz/](https://dev.helius.xyz/)
2. Sign up or log in (free tier available)
3. Create a new project  
4. Copy your API key

### List All Secrets

```bash
cd laserstream-container
wrangler secret list
```

### Delete a Secret

```bash
cd laserstream-container
wrangler secret delete SECRET_NAME
```

## Adding Custom Scripts

For complex automation, add scripts to this directory:

1. Create the script file (Node.js recommended for cross-platform compatibility)
2. Add to root `package.json` under `scripts`:

   ```json
   "your-command": "node scripts/your-script.js"
   ```

### `setup-secrets.sh`

Automates setting up Cloudflare secrets for the LaserStream container.

**Usage**:

```bash
# From repository root
pnpm run setup:secrets
```

**What it does**:

1. Checks if `.env` file exists
2. Loads environment variables from `.env`
3. Sets `HELIUS_API_KEY` secret in Cloudflare Workers via wrangler
4. Prompts for manual input if values are missing

**Prerequisites**:

- `.env` file with `HELIUS_API_KEY` set
- Wrangler CLI installed
- Authenticated with Cloudflare (`wrangler login`)

## Adding New Scripts

To add a new script:

1. Create the script file in `scripts/`
2. Make it executable: `chmod +x scripts/your-script.sh`
3. Add to root `package.json` under `scripts`:

   ```json
   "your-command": "./scripts/your-script.sh"
   ```

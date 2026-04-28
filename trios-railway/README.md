# trios-railway MCP Server

[![CI](https://github.com/gHashTag/trios-railway/workflows/CI/badge.svg)](https://github.com/gHashTag/trios-railway/actions)
[![npm](https://img.shields.io/npm/v/@ghashtag/trios-railway-mcp)](https://www.npmjs.com/package/@ghashtag/trios-railway-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

MCP (Model Context Protocol) server for Railway service management with Streamable HTTP (SSE) transport.

## Installation

```bash
npm install @ghashtag/trios-railway-mcp
```

## Features

- **6 Railway Management Tools**: Deploy, list, delete, redeploy services
- **Streamable HTTP (SSE)**: Session-based MCP protocol 2024-11-05
- **Health Check**: `/health` endpoint for monitoring
- **Input Validation**: All tool parameters validated
- **GitHub Actions CI/CD**: Automated releases

## Tools

| Tool | Description |
|------|-------------|
| `railway_service_deploy` | Deploy/redeploy Railway service with environment variable upsert |
| `railway_service_list` | List all Railway services in IGLA project |
| `railway_service_delete` | Delete Railway services by ID (requires `confirm: true`) |
| `railway_service_redeploy` | Redeploy existing Railway services |
| `railway_audit_migrate_sql` | Generate Neon DDL for audit tables |
| `railway_experience_append` | Append structured lines to L7 experience log |

## Usage

### Perplexity AI Connector

1. In Perplexity, add a Custom Connector:
   - **Authentication**: API Key
   - **API Key**: `your-railway-token`
   - **MCP Server URL**: `https://trios-mcp-public-production.up.railway.app/mcp`
   - **Transport**: Streamable HTTP

2. Click Add and start using Railway tools!

### Local Development

```bash
# Clone repository
git clone https://github.com/gHashTag/trios-railway.git
cd trios-railway

# Install dependencies
npm install

# Build
npm run build

# Run locally
PORT=3026 RAILWAY_API_TOKEN=your-token node dist/index.js
```

### Claude Desktop Configuration

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "trios-railway": {
      "url": "https://trios-mcp-public-production.up.railway.app/mcp",
      "transport": "streamable-http"
    }
  }
}
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `RAILWAY_API_TOKEN` | Yes | Railway API token for service management |
| `RAILWAY_PROJECT_ID` | No | Railway project ID (defaults to IGLA project) |
| `PORT` | No | Server port (defaults to 3026) |

## Deployment

### Railway

The server is deployed at: https://trios-mcp-public-production.up.railway.app/mcp

### Self-Hosted

```bash
# Using Docker
docker build -t trios-railway-mcp .
docker run -p 3026:3026 -e RAILWAY_API_TOKEN=your-token trios-railway-mcp
```

## Health Check

```bash
curl https://trios-mcp-public-production.up.railway.app/health
```

Response:
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "tools": 6
}
```

## Example: Deploy a Railway Service

```json
{
  "name": "railway_service_deploy",
  "arguments": {
    "name": "my-service",
    "env": {
      "NODE_ENV": "production"
    }
  }
}
```

## License

MIT © [gHashTag](https://github.com/gHashTag)

## Links

- [GitHub](https://github.com/gHashTag/trios-railway)
- [npm](https://www.npmjs.com/package/@ghashtag/trios-railway-mcp)
- [CHANGELOG](CHANGELOG.md)
- [MCP Protocol](https://modelcontextprotocol.io)

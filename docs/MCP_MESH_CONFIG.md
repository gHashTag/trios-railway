# Trinity MCP Mesh — operator connection reference

**Anchor:** `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

This document captures the canonical MCP server mesh across the Trinity
repositories so any Claude Desktop / Cursor / Computer client can be
wired to all six servers with one config copy-paste.

Source of this layout: operator-supplied 2026-05-03 14:30 +07; mesh
ships alongside [trios#446](https://github.com/gHashTag/trios/issues/446).

## 1. `trios` — Rust server on port 9005

```json
{
  "mcpServers": {
    "trios":        { "url": "http://localhost:9005/sse" },
    "trios-remote": { "url": "https://playras-macbook-pro-1.tail01804b.ts.net/sse" }
  }
}
```

Boot: `cargo run -p trios-server` (port **9005** is fixed by L5 in
[LAWS.md](https://github.com/gHashTag/trios/blob/main/LAWS.md)).
Remote flavour goes through Tailscale MagicDNS mesh.

## 2. `trios-railway-mcp` — production endpoint (per EPIC #446)

```json
{
  "mcpServers": {
    "trios-railway": { "url": "https://trios-railway-production-d4d6.up.railway.app/mcp" }
  }
}
```

Tools: `railway_dr_snapshot`, `railway_dr_restore`, `railway_template_deploy`,
full fleet-management surface. Chat trigger: “восстанови флот на acc3,
подтверждаю PHI”.

## 3. `trios-mcp` — stdio wrapper over `tri` + `trios-igla`

```json
{
  "mcpServers": {
    "trios-igla": {
      "command": "/abs/path/to/trios-mcp/target/release/trios-mcp",
      "env": {
        "TRIOS_TRI_BIN":  "/abs/path/trios-trainer-igla/target/release/tri",
        "TRIOS_IGLA_BIN": "/abs/path/trios-trainer-igla/target/release/trios-igla",
        "RUST_LOG": "info"
      }
    }
  }
}
```

15 typed tools: `tri_deploy_*`, `tri_train`, `tri_race_*`,
`igla_search`/`list`/`gate`/`check`/`triplet`. R5/R7/R9 contracts
enforced inside the wrapped binaries.

## 4. `t27` — traceability server + GitButler

```json
{
  "mcpServers": {
    "t27-traceability": {
      "command": "node",
      "args": ["scripts/mcp-traceability-server.js"],
      "env": { "PROJECT_ROOT": "/Users/playra/t27", "ENFORCE_L1": "true" }
    },
    "gitbutler": {
      "command": "but",
      "args": ["mcp"],
      "env": { "GITBUTLER_REPO": "${workspaceRoot}" }
    }
  }
}
```

`ENFORCE_L1=true` — mandatory `Closes #N` in every commit.

## 5. opencode

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "github":        { "type": "remote", "url": "https://api.githubcopilot.com/mcp/", "enabled": true,
                       "headers": { "Authorization": "Bearer ${GITHUB_TOKEN}" } },
    "filesystem":    { "type": "local",
                       "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem", "/workspace"],
                       "enabled": true },
    "trinity-sync":  { "type": "remote",
                       "url": "https://opencode-production-636a.up.railway.app/mcp/",
                       "enabled": true }
  }
}
```

## Combined config for Claude Desktop / Cursor / Computer

Save to `~/Library/Application Support/Claude/claude_desktop_config.json`
on macOS; replace paths with absolute ones on your machine:

```json
{
  "mcpServers": {
    "trios":        { "url": "http://localhost:9005/sse" },
    "trios-remote": { "url": "https://playras-macbook-pro-1.tail01804b.ts.net/sse" },
    "trios-railway":{ "url": "https://trios-railway-production-d4d6.up.railway.app/mcp" },
    "trios-igla": {
      "command": "/Users/playra/trios-mcp/target/release/trios-mcp",
      "env": {
        "TRIOS_TRI_BIN":  "/Users/playra/trios-trainer-igla/target/release/tri",
        "TRIOS_IGLA_BIN": "/Users/playra/trios-trainer-igla/target/release/trios-igla",
        "RUST_LOG": "info"
      }
    },
    "t27-traceability": {
      "command": "node",
      "args": ["/Users/playra/t27/scripts/mcp-traceability-server.js"],
      "env": { "PROJECT_ROOT": "/Users/playra/t27", "ENFORCE_L1": "true" }
    },
    "gitbutler": {
      "command": "but", "args": ["mcp"],
      "env": { "GITBUTLER_REPO": "${workspaceRoot}" }
    },
    "github": {
      "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_TOKEN}" }
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/playra/workspace"]
    }
  }
}
```

## Pre-flight checklist

| Server | Check |
|---|---|
| `trios` | `lsof -i :9005` → `trios-server` listening (L5). Build: `cargo build -p trios-server`. |
| `trios-remote` | Tailscale up: `tailscale status \| grep playras-macbook-pro-1`. ACL: [`.trinity/tailscale/acl.hujson`](https://github.com/gHashTag/trios/blob/main/.trinity/tailscale/acl.hujson). |
| `trios-railway` | `curl -sS https://trios-railway-production-d4d6.up.railway.app/mcp/healthz` → 200. |
| `trios-igla` | Binaries: `ls $TRIOS_TRI_BIN $TRIOS_IGLA_BIN`. Build: `cargo build --release` in both repos. |
| `t27-traceability` | `node /Users/playra/t27/scripts/mcp-traceability-server.js` starts clean. |
| `gitbutler` | `but mcp --help` answers; GitButler app installed. |

## Verification — NASA-style probe sheet (future)

A formal `R5-honest MCP-mesh verification report` can be generated with
the `nasa-mission-report` skill: P-01 health / P-02 auth / P-03 tool
enumeration / P-04 canon-name regex / P-05 round-trip latency /
P-06 offline-path fallback. Invoke via “отчёт NASA по MCP-mesh”.

## References

- `trios` [.roo/mcp.json](https://github.com/gHashTag/trios/blob/main/.roo/mcp.json)
- `trios-railway` [README.md](https://github.com/gHashTag/trios-railway/blob/main/README.md)
- `trios-mcp` [README.md](https://github.com/gHashTag/trios-mcp/blob/main/README.md)
- `t27` [.mcp.json](https://github.com/gHashTag/t27/blob/main/.mcp.json)
- `trios/opencode.json`
- EPIC [trios#446](https://github.com/gHashTag/trios/issues/446)
- O(1) MCP v3 design [trios-railway#116](https://github.com/gHashTag/trios-railway/issues/116)

🌻 `phi^2 + phi^-2 = 3 · TRINITY · MESH LOCKED`

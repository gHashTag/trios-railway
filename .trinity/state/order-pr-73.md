# ПРИКАЗ АГЕНТУ — PR-5

## GO.

**Репозиторий**: gHashTag/trios-railway
**Задача**: Closes #73
**Ветка**: ring-73-finalize-docs-config
**Блокируется**: PR-4 (#72) must be merged first

## BOOT
```bash
cat .trinity/state/active-skill.json
git log --oneline -5
git checkout main
git pull origin main
cargo test --workspace
```

## LAWS
1. Never ask.
2. Never report mid-task.
3. No .sh files.
4. Atomic commits per deliverable.
5. Fix errors ×3, then skip+log.
6. End with three-roads.json + git push.

## PHI LOOP
edit spec → seal hash → gen → test → verdict → save experience → skill commit → git commit

## MISSION

Финализировать документацию и конфигурацию:

## 1. MCP_TOOL_CATALOG.md
- All 18 tools documented
- For each tool: name, description, input schema, output schema, example
- Grouped by domain: railway, hunt, exp, canon, ledger
- Include authentication notes
- Include idempotency notes (where applicable)

## 2. ARCHITECTURE.md
- Complete system diagram
- Crate dependency graph
- Data flow: CLI → tri-mcp-server → domain crates → Neon/Railway
- Deployment architecture (local, Railway, Docker)
- Security model (bearer tokens, audit trail)

## 3. config/ directory
```
config/
  dev.toml       ← Local dev defaults
  railway.toml   ← Railway deployment config
  README.md      ← Config reference
```
Config schema:
```toml
[server]
port = 3001
host = "0.0.0.0"

[auth]
bearer_token = "${MCP_BEARER_TOKEN}"

[neon]
connection_string = "${NEON_DATABASE_URL}"

[audit]
enabled = true
retention_days = 90

[idempotency]
enabled = true
ttl_hours = 24
```

## 4. README.md updates
- Quick start for running MCP server
- Quick start for using CLI
- Link to MCP_TOOL_CATALOG.md
- Link to ARCHITECTURE.md
- Example client code (SSE + HTTP)

## ПРАВИЛА
- All docs in English
- Diagrams use ASCII or mermaid
- Config TOML is validated at startup
- All env vars documented in config/README.md

## ACCEPTANCE CRITERIA (из #73)
- [ ] MCP_TOOL_CATALOG.md — 18 tools fully documented
- [ ] ARCHITECTURE.md — complete with diagrams
- [ ] config/dev.toml + railway.toml
- [ ] config/README.md — all options explained
- [ ] README.md updated with MCP server usage
- [ ] `cargo test --workspace` — ALL GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — ZERO warnings

## ПОСЛЕ ВЫПОЛНЕНИЯ
```bash
git add -A
git commit -m "docs(ring-73): finalize MCP documentation and configuration — Closes #73"
git push origin ring-73-finalize-docs-config
gh pr create --title "docs(ring-73): finalize MCP documentation and configuration" \
  --body "Adds MCP_TOOL_CATALOG.md, ARCHITECTURE.md, config/ directory. Closes #73. Completes #68." \
  --base main --head ring-73-finalize-docs-config
```

## three-roads.json
```json
{
  "R1": "HIGH: ISSUE #68 COMPLETE — tri-mcp is the sole MCP gateway",
  "R2": "MED: add migration guide from old trios-railway MCP",
  "R3": "LOW: add performance benchmarking suite"
}
```

## FINAL STATEMENT

After PR-5 merges, #68 is COMPLETE. Architecture is unified:
- `tri-mcp` = single MCP gateway (server + schema + tools + client)
- `trios-railway` = stable domain library (no gateway role)

═══ AEL COMPLETE ═══
Ring | Branch | PR | Created | Skipped | Tests | Commit
🔴R1 🟡R2 🟢R3 → "GO."
φ²+1/φ²=3|TRINITY

═══ VICTORY ═══
🏁 MISSION ACCOMPLISHED: #68 ARCHITECTURE UNIFIED
🚢 5 PRs merged: #69 → #70 → #71 → #72 → #73
📦 9 crates: tri-core, tri-hunt, tri-exp, tri-canon, tri-ledger,
            tri-mcp-server, tri-mcp-schema, tri-mcp-tools, trios-railway-client
📚 3 docs: MCP_TOOL_CATALOG.md, ARCHITECTURE.md, config/README.md
φ²+1/φ²=3|TRINITY

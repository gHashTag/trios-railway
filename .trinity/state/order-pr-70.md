# ПРИКАЗ АГЕНТУ — PR-2

## GO.

**Репозиторий**: gHashTag/trios-railway
**Задача**: Closes #70
**Ветка**: ring-70-add-mcp-workspace
**Блокируется**: PR-1 (#69) must be merged first

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
4. Atomic commits per crate.
5. Fix errors ×3, then skip+log.
6. End with three-roads.json + git push.

## PHI LOOP
edit spec → seal hash → gen → test → verdict → save experience → skill commit → git commit

## MISSION

В workspace добавить 4 MCP crates:

```
crates/tri-mcp-server/   ← Axum server on port 3001, SSE/stdio transport
                           Depends on: tri-core, tri-hunt, tri-exp, tri-canon, tri-ledger

crates/tri-mcp-schema/   ← Zod schemas for all tools, request/response types
                           Generated from: tri-core/hunt/exp/canon/ledger pub API

crates/tri-mcp-tools/    ← Tool implementations, 1:1 mapping to domain crate functions
                           railway.service.{list,deploy,kill,restart,logs,env}
                           hunt.{seed,smoke,rung,prune,mirror}
                           exp.{next,claim,list}
                           canon.{validate,tripwire_check}

crates/trios-railway-client/ ← HTTP client wrapper around tri-mcp-server
                           For CLI tools that call MCP server instead of direct libs
```

## ПРАВИЛА
- tri-mcp-server слушает на 0.0.0.0:3001, SSE endpoint /sse
- Все tool-ы возвращают типизированные результаты через Zod
- tri-mcp-schema = source of truth для типов — остальные crates импортируют
- Zero deps между MCP crates кроме tri-mcp-server → others
- Все MCP tools имеют `description` и `inputSchema` из tri-mcp-schema

## ACCEPTANCE CRITERIA (из #70)
- [ ] crates/tri-mcp-server/ — Axum + SSE, /sse endpoint working
- [ ] crates/tri-mcp-schema/ — Zod schemas for all 20+ tools
- [ ] crates/tri-mcp-tools/ — implementations wired to domain crates
- [ ] crates/trios-railway-client/ — HTTP client wrapper
- [ ] `cargo build -p tri-mcp-*` — ALL PASS
- [ ] `cargo test --workspace` — ALL GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — ZERO warnings
- [ ] SSE endpoint emits valid MCP Server-Sent Events

## ПОСЛЕ ВЫПОЛНЕНИЯ
```bash
git add -A
git commit -m "feat(ring-70): add tri-mcp workspace crates — Closes #70"
git push origin ring-70-add-mcp-workspace
gh pr create --title "feat(ring-70): add MCP workspace crates" \
  --body "Adds tri-mcp-server, tri-mcp-schema, tri-mcp-tools, trios-railway-client. Closes #70. Part of #68." \
  --base main --head ring-70-add-mcp-workspace
```

## three-roads.json
```json
{
  "R1": "HIGH: proceed to #71 — register mcp.* tools in server",
  "R2": "MED: add E2E tests for MCP tool invocations",
  "R3": "LOW: add WebSocket transport as alternative to SSE"
}
```

═══ AEL COMPLETE ═══
Ring | Branch | PR | Created | Skipped | Tests | Commit
🔴R1 🟡R2 🟢R3 → "GO."
φ²+1/φ²=3|TRINITY

# ПРИКАЗ АГЕНТУ — PR-3

## GO.

**Репозиторий**: gHashTag/trios-railway
**Задача**: Closes #71
**Ветка**: ring-71-register-tools
**Блокируется**: PR-2 (#70) must be merged first

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
4. Atomic commits per tool-group.
5. Fix errors ×3, then skip+log.
6. End with three-roads.json + git push.

## PHI LOOP
edit spec → seal hash → gen → test → verdict → save experience → skill commit → git commit

## MISSION

Зарегистрировать все MCP tools в tri-mcp-server:

## Tool Groups to Register

### railway.* (6 tools)
```
mcp.railway.service.list
mcp.railway.service.deploy
mcp.railway.service.kill
mcp.railway.service.restart
mcp.railway.service.logs
mcp.railway.service.env.list
mcp.railway.service.env.set
```

### hunt.* (5 tools)
```
mcp.hunt.seed.status
mcp.hunt.seed.smoke_race
mcp.hunt.seed.rung_schedule
mcp.hunt.seed.prune_diverging
mcp.hunt.seed.mirror_siblings
```

### exp.* (3 tools)
```
mcp.exp.next_id
mcp.exp.claim_ids
mcp.exp.list
```

### canon.* (2 tools)
```
mcp.canon.validate
mcp.canon.tripwire_check
```

### ledger.* (2 tools)
```
mcp.ledger.append
mcp.ledger.query
```

## ПРАВИЛА
- Tool names follow pattern: `mcp.{domain}.{entity}.{action}`
- Все tools используют схемы из tri-mcp-schema
- Ошибки из domain crates обёртываются в MCP ToolError
- Каждый tool имеет meaningful description
- Server capabilities advertise all 18 tools

## ACCEPTANCE CRITERIA (из #71)
- [ ] All 18 tools registered in tri-mcp-server
- [ ] `GET /sse` returns tools/list response
- [ ] `POST /sse` with tools/call works for all tools
- [ ] Error handling: domain errors → MCP error responses
- [ ] `cargo test --workspace` — ALL GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — ZERO warnings
- [ ] Manual SSE client can invoke all tools

## ПОСЛЕ ВЫПОЛНЕНИЯ
```bash
git add -A
git commit -m "feat(ring-71): register all MCP tools in server — Closes #71"
git push origin ring-71-register-tools
gh pr create --title "feat(ring-71): register MCP tools in server" \
  --body "Registers railway.*, hunt.*, exp.*, canon.*, ledger.* tools (18 total). Closes #71. Part of #68." \
  --base main --head ring-71-register-tools
```

## three-roads.json
```json
{
  "R1": "HIGH: proceed to #72 — add bearer auth + audit + idempotency",
  "R2": "MED: add tool invocation logging to tri-ledger",
  "R3": "LOW: add tool metrics (call count, latency histogram)"
}
```

═══ AEL COMPLETE ═══
Ring | Branch | PR | Created | Skipped | Tests | Commit
🔴R1 🟡R2 🟢R3 → "GO."
φ²+1/φ²=3|TRINITY

# ПРИКАЗ АГЕНТУ — PR-4

## GO.

**Репозиторий**: gHashTag/trios-railway
**Задача**: Closes #72
**Ветка**: ring-72-auth-audit-idempotency
**Блокируется**: PR-3 (#71) must be merged first

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
4. Atomic commits per feature.
5. Fix errors ×3, then skip+log.
6. End with three-roads.json + git push.

## PHI LOOP
edit spec → seal hash → gen → test → verdict → save experience → skill commit → git commit

## MISSION

Добавить три критических feature в tri-mcp-server:

## 1. Bearer Authentication
- Env var: `MCP_BEARER_TOKEN`
- Header: `Authorization: Bearer <token>`
- All /sse endpoints require valid token
- 401 response on missing/invalid token
- Skip auth if `MCP_BEARER_TOKEN` not set (dev mode)

## 2. Audit Ledger Integration
- Every tool invocation → record in tri-ledger
- Schema: `mcp_tool_invocation` table
- Fields: tool_name, args_hash, result_status, duration_ms, timestamp
- Append-only, never truncate (L7)
- Use tri-ledger crate for all writes

## 3. Idempotency Keys
- Header: `Idempotency-Key: <uuid>`
- Store: `mcp_idempotency` table (Neon)
- Logic:
  - If key seen → return cached result
  - If key new → execute + cache result
- TTL: 24 hours
- Required for write tools: deploy, kill, restart, env.set

## ПРАВИЛА
- Auth middleware wraps all MCP routes
- Audit logging happens before response return
- Idempotency check happens before tool execution
- Cached results include full MCP response

## ACCEPTANCE CRITERIA (из #72)
- [ ] Bearer auth working, 401 on bad token
- [ ] Every tool call → audit ledger entry
- [ ] Idempotency keys prevent duplicate writes
- [ ] `cargo test --workspace` — ALL GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — ZERO warnings
- [ ] Integration tests for auth, audit, idempotency

## ПОСЛЕ ВЫПОЛНЕНИЯ
```bash
git add -A
git commit -m "feat(ring-72): add bearer auth, audit ledger, idempotency — Closes #72"
git push origin ring-72-auth-audit-idempotency
gh pr create --title "feat(ring-72): add auth, audit, idempotency" \
  --body "Adds Bearer authentication, audit ledger integration, idempotency keys. Closes #72. Part of #68." \
  --base main --head ring-72-auth-audit-idempotency
```

## three-roads.json
```json
{
  "R1": "HIGH: proceed to #73 — finalize docs + config",
  "R2": "MED: add rate limiting per token",
  "R3": "LOW: add admin endpoints for audit ledger query"
}
```

═══ AEL COMPLETE ═══
Ring | Branch | PR | Created | Skipped | Tests | Commit
🔴R1 🟡R2 🟢R3 → "GO."
φ²+1/φ²=3|TRINITY

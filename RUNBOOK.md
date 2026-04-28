# trios-railway-mcp — Disaster Recovery Runbook

One MCP gateway. Four operator accounts. Multi-account routing.
Pull-based experiment loop (ADR-0081). Anchor: `phi^2 + phi^-2 = 3`.

This runbook is the **single source of truth** for restoring the MCP
gateway and the seed-agent fleet from scratch. Every step is
copy-pasteable. R5-honest: every command's expected output is documented;
deviations surface immediately.

---

## 0. What this system is

```text
┌─────────────────────────────────────────────────────────────────────┐
│ trios-railway-mcp (one Streamable-HTTP server)                      │
│                                                                     │
│   trios-railway-production.up.railway.app/mcp                       │
│                                                                     │
│   15 tools: 6 legacy railway_*  +  9 curated mcp.<domain>.<verb>    │
│   8 tripwires (#107..#114) at every entry                           │
│   RailwayMultiClient routes per-call to acc0/acc1/acc2/acc3         │
└─────────────────────────────────────────────────────────────────────┘
                          │                          │
                          ▼                          ▼
                  Railway GraphQL API          Neon (Postgres)
                  (4 account tokens)            ADR-0081 schema:
                                                  experiment_queue
                                                  bpb_samples
                                                  workers
                                                  gardener_decisions
                                                  v_leaderboard
                                                  + audit ledger
                                                       ▲
                                                       │
                                            ┌──────────┴──────────┐
                                            │ seed-agent workers   │
                                            │ (Railway services)   │
                                            │                      │
                                            │ pull → train →       │
                                            │ early-stop @ 1000 →  │
                                            │ loop                 │
                                            └──────────────────────┘
```

---

## 1. Critical resources

| Asset | Location |
|---|---|
| Source repo | `gHashTag/trios-railway` (main) |
| Default branch | `main` (HEAD `afbb108` at v1.1.0 cut) |
| Backup of pre-consolidation main | branch `archive/main-pre-consolidation-2026-04-28` |
| MCP image | `ghcr.io/ghashtag/trios-railway-mcp:multi-acc` (public) |
| seed-agent image | `ghcr.io/ghashtag/trios-seed-agent:latest` (public) |
| MCP service | Railway project IGLA `e4fe33bb-3b09-4842-9782-7d2dea1abc9b`, service `b84f7b81-fbc3-4de4-8df9-edcdc16ed399`, env `54e293b9-00a9-4102-814d-db151636d96e` |
| Public URL | `https://trios-railway-production.up.railway.app/mcp` |
| Health probe | `https://trios-railway-production.up.railway.app/healthz` → `ok` |
| Neon database | `$NEON_DATABASE_URL` (set in operator's keychain) |
| Backup files | `.trinity/trios_railway_full_ddl_neon.sql` (full DDL) and `.trinity/phase_e_seed_batch_acc1.sql` (smoke-first seed batch) |

**Operator accounts (Railway):**

| Alias | Email | Project (whitelisted) |
|---|---|---|
| `acc0` | kaglerslomaansc@hotmail.com | `265301ce-0bf2-4187-a36f-348b0eb9942f` (trios-trainer) |
| `acc1` | rumbodzalaclhdv0@hotmail.com | `e4fe33bb-3b09-4842-9782-7d2dea1abc9b` (IGLA — current race) |
| `acc2` | brabbtjubindt5cug@hotmail.com | `39d833c1-4cb6-4af9-b61b-c204b6733a98` (thriving-eagerness) |
| `acc3` | gondiigamzevup@hotmail.com | (project not yet in `ALLOWED_PROJECT_IDS`) |

`ALLOWED_PROJECT_IDS` is a const in
`crates/trios-railway-core/src/multiclient.rs`. Tripwire #107 rejects
any mutation against a project not in this list.

---

## 2. Health check (verify gateway is alive)

```bash
# 1. HTTP root
curl -fsS https://trios-railway-production.up.railway.app/             # → "trios-railway-mcp: public MCP server..."
curl -fsS https://trios-railway-production.up.railway.app/healthz      # → "ok"

# 2. MCP handshake (initialize → tools/list)
BASE=https://trios-railway-production.up.railway.app/mcp
RESP=$(curl -s -i -X POST "$BASE" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-03-26","capabilities":{},
        "clientInfo":{"name":"healthcheck","version":"0"}}}')
SESSION=$(echo "$RESP" | awk -F': ' '/^mcp-session-id/{print $2}' | tr -d '\r')
echo "session=$SESSION"

curl -s -X POST "$BASE" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | grep -oP '"name":"[^"]+"' | sort -u
# Expected: 15 distinct tool names — 6 railway_* + 9 mcp.*
```

If `/healthz` does NOT return `ok` → gateway is down. Go to §4.

---

## 3. From a clean clone — bring everything up in 4 steps

```bash
# 1. Clone & verify HEAD
git clone https://github.com/gHashTag/trios-railway
cd trios-railway
git log --oneline -3
# Expected: top commit "ci: railway.toml points at Dockerfile.mcp" or newer.

# 2. Reproduce the test suite (no Neon, no Railway).
cargo test --workspace -- --test-threads=1
# Expected: 173/173 GREEN (or higher).

# 3. Build the MCP server image locally (smoke).
docker build -f Dockerfile.mcp -t trios-railway-mcp:local .
docker run --rm -e PORT=8080 -p 8080:8080 trios-railway-mcp:local &
curl -fsS http://localhost:8080/healthz; kill %1
# Expected: "ok"

# 4. Build the seed-agent image (smoke).
docker build -f Dockerfile.seed-agent -t trios-seed-agent:local .
# (no port — pull worker, exits cleanly without NEON_DATABASE_URL)
```

---

## 4. Restore the live gateway from scratch

If `b84f7b81` is corrupted, deleted, or stuck, replace it:

```bash
# A. Confirm GHCR images are public (they should be).
curl -sI -H "Authorization: Bearer $(curl -s 'https://ghcr.io/token?scope=repository:ghashtag/trios-railway-mcp:pull' | jq -r .token)" \
  "https://ghcr.io/v2/ghashtag/trios-railway-mcp/manifests/multi-acc"
# Expected: HTTP/2 200

# B. From Railway dashboard for the IGLA project:
#    - Settings → Source → "gHashTag/trios-railway", branch "main".
#      (railway.toml in the repo points the build at Dockerfile.mcp.)
#    - Variables (Raw Editor) — paste from .trinity/railway_vars_b84f7b81.txt
#      filling 7 placeholders:
#        RAILWAY_TOKEN_ACC{0,1,2,3}, RAILWAY_PROJECT_ID_ACC3,
#        RAILWAY_ENVIRONMENT_ID_ACC3, NEON_DATABASE_URL.
#    - Save → Deploy.
#
# C. Wait ~3 minutes for Rust build. Then re-run §2 health check.
```

If GHCR pull is failing with `unable to connect to the registry`,
the package visibility was flipped back to private. Re-flip at:
[github.com/users/gHashTag/packages/container/trios-railway-mcp/settings](https://github.com/users/gHashTag/packages/container/trios-railway-mcp/settings)
→ Danger Zone → Change visibility → Public.

---

## 5. Restore the Neon schema

If the Neon database was reset / migrated / new branch:

```bash
# Single-shot: reset+apply the full ADR-0081 + audit-ledger schema.
psql "$NEON_DATABASE_URL" -f .trinity/trios_railway_reset_and_apply.sql
# Expected (last lines):
#   table_name | (9 rows: experiment_queue, bpb_samples, workers,
#                  gardener_decisions, gardener_runs, railway_*)
#   viewname   | (2 rows: v_leaderboard, v_railway_drift_open)
#   counts     | (4 rows, all 0)
```

Or via Neon dashboard:
1. [console.neon.tech](https://console.neon.tech/) → IGLA project → SQL Editor
2. Paste contents of `.trinity/trios_railway_reset_and_apply.sql`
3. Run → expect "Statement executed successfully" with 4 verification rows.

---

## 6. Bootstrap the experiment queue

After the schema is in place but before deploying any seed-agent:

```bash
# Apply the smoke-first seed batch — 3 train_v2 champion-config rows
# at priority=0 for Acc1.
psql "$NEON_DATABASE_URL" -f .trinity/phase_e_seed_batch_acc1.sql

# Verify
psql "$NEON_DATABASE_URL" -c "
  SELECT canon_name, seed, priority, status, steps_budget
  FROM experiment_queue
  ORDER BY priority ASC, seed ASC;"
# Expected: 3 rows, all priority=0, status='pending', steps_budget=120000
```

---

## 7. Deploy a seed-agent worker

Through the live MCP gateway (preferred — single source of truth):

```jsonc
// mcp.railway.deploy with idempotency + account scoping (#112, #114)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "mcp.railway.deploy",
    "arguments": {
      "idempotency_key": "deploy-seed-agent-001-2026-04-28",
      "account": "acc1",
      "name": "seed-agent-001",
      "image": "ghcr.io/ghashtag/trios-seed-agent:latest",
      "vars": [
        {"key": "NEON_DATABASE_URL", "value": "<from-keychain>"},
        {"key": "RAILWAY_ACC", "value": "acc1"},
        {"key": "RAILWAY_SERVICE_NAME", "value": "seed-agent-001"},
        {"key": "RUST_LOG", "value": "info"}
      ]
    }
  }
}
```

`RAILWAY_SERVICE_ID` is supplied by Railway as `$RAILWAY_SERVICE_ID`
automatically — no need to set it manually.

Verify the worker is alive:

```sql
-- one row per worker
SELECT id, railway_acc, railway_svc_name, last_heartbeat, current_exp_id
FROM workers ORDER BY registered_at DESC LIMIT 5;

-- claimed experiment (status flips to claimed → running)
SELECT canon_name, seed, status, claimed_at, started_at, worker_id
FROM experiment_queue WHERE status IN ('claimed','running') LIMIT 5;

-- BPB telemetry (rows appear every 100 trainer steps)
SELECT canon_name, seed, step, bpb FROM bpb_samples
ORDER BY ts DESC LIMIT 10;
```

---

## 8. Common failure modes

| Symptom | Cause | Fix |
|---|---|---|
| `Container failed to start: unable to connect to the registry` | GHCR package became private | Re-flip to public (§4 last paragraph) |
| `tools/list` returns only 6 `railway_*` tools | Connector points at old single-account image | Re-register custom remote in Perplexity → URL `…/mcp` from §1 |
| `mcp.railway.deploy` returns `tripwire #107: project … not in ALLOWED_PROJECT_IDS` | Project not whitelisted | Add UUID to `crates/trios-railway-core/src/multiclient.rs::ALLOWED_PROJECT_IDS`, push, rebuild |
| `mcp.fleet.snapshot` returns `Not Authorized` for an account | Token is workspace-scoped or expired | Re-issue Personal API Token under that account (railway.com/account/tokens), set `RAILWAY_TOKEN_ACC{N}` and `RAILWAY_TOKEN_KIND_ACC{N}=team` (or `project`) |
| `seed-agent` crashes on boot with `42P01 relation does not exist` | Neon schema not applied | Re-run §5 |
| Workers register but never claim | All experiments are `done`/`pruned` | Run §6 to add new rows, or have gardener `strategy-tick` enqueue follow-ups |
| `experiment_queue` rows stuck in `claimed` for > 5 min | Worker crashed without releasing | Gardener `strategy-tick` emits `ResetStaleClaim` automatically; worst case `UPDATE … SET status='pending', worker_id=NULL WHERE status='claimed' AND claimed_at < now() - interval '5 min'` |

---

## 9. Roll back the consolidation force-push

If something goes wrong with main and you need yesterday's state:

```bash
# Backup branch was created at force-push time.
git fetch origin
git checkout archive/main-pre-consolidation-2026-04-28
git push origin +HEAD:main      # destructive — restores the legacy main.
```

Note: legacy main has only `Dockerfile.mcp` + cherry-picked workflow,
zero MCP source code. Use only as last resort while diagnosing.

---

## 10. Test counts at each merge point

Useful for spotting regressions after a migration:

| Tag / branch | Total tests |
|---|---|
| pre-PR-77 baseline | 70 |
| PR #77 (RailwayMultiClient) | 82 |
| PR #79 (canon-shared-core) | 97 |
| PR #80 (mcp aliases + tripwires) | 134 |
| PR #82 (experiment_queue DDL) | 140 |
| PR #83 (seed-agent) | 163 |
| PR #84 (gardener strategy-tick) | 173 |
| **v1.1.0** | **173** GREEN |

Run with `cargo test --workspace -- --test-threads=1` (the
`from_env_skips_empty_slots` env-var test is order-sensitive under
parallel runs; this is a pre-existing flake, not a regression).

---

## 11. Where to file issues

- **Architectural questions** → [trios-railway#81](https://github.com/gHashTag/trios-railway/issues/81) (ADR-0081)
- **Token / auth issues** → [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61)
- **Neon DDL Pipedream rollback** → [trios-railway#62](https://github.com/gHashTag/trios-railway/issues/62) (workaround in this runbook §5)
- **Race / leaderboard** → [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)

---

`phi^2 + phi^-2 = 3 · TRINITY · ONE-GATEWAY · FOUR-TOKENS · NEVER-STOP`

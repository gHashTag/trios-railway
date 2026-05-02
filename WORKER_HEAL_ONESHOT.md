# WORKER_HEAL_ONESHOT.md

**Цель:** Диагностировать и излечить fleet (8 Railway accounts × scarabs + Neon writer).

**Anchor:** `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP — diagnose, document, heal where possible.`

**LAST UPDATE:** 2026-05-01 04:10 UTC — Confirmed trainer checkpoint stall at step 1000, fixed STEP 2 schema query.

**⚠️ CRITICAL LIMITATION:** Project-scoped Railway tokens (`RAILWAY_TOKEN_ACC*`) **DO NOT** support GraphQL/REST API access. Cannot restart services programmatically. Need manual intervention or user-level token.

**Read this file once. Execute top-to-bottom. Do not skip steps.**

***

## STEP 0 — Load credentials (10 sec)

```bash
cd /Users/playra/trios-railway
set -a; source .env; set +a

# Verify required tokens are loaded (ACTUAL .env variable names)
: "${NEON_DATABASE_URL:?NEON_DATABASE_URL missing in .env}"

# Per-account Railway tokens (project-scoped, limited permissions)
: "${RAILWAY_TOKEN_ACC0:?RAILWAY_TOKEN_ACC0 missing in .env}"
: "${RAILWAY_TOKEN_ACC1:?RAILWAY_TOKEN_ACC1 missing in .env}"
: "${RAILWAY_TOKEN_ACC2:?RAILWAY_TOKEN_ACC2 missing in .env}"
: "${RAILWAY_TOKEN_ACC3:?RAILWAY_TOKEN_ACC3 missing in .env}"
: "${RAILWAY_TOKEN_ACC4:?RAILWAY_TOKEN_ACC4 missing in .env}"
: "${RAILWAY_TOKEN_ACC5:?RAILWAY_TOKEN_ACC5 missing in .env}"
: "${RAILWAY_TOKEN_ACC6:?RAILWAY_TOKEN_ACC6 missing in .env}"
: "${RAILWAY_TOKEN_ACC7:?RAILWAY_TOKEN_ACC7 missing in .env}"

# Optional: GitHub token for image rebuild (STEP 6)
# : "${GITHUB_TOKEN:?GITHUB_TOKEN missing in .env}"

echo "✅ All tokens loaded"
```

**Stop on failure.** Не двигаться дальше если хотя бы один токен пустой.

***

## STEP 1 — Diagnose root cause via Neon (read-only, 30 sec)

**⚠️ SCHEMA NOTES:**
- `scarabs.railway_acc` (not `account`)
- `strategy_queue` does NOT have `timeout_seconds` column (handled by worker code, not DB)

```bash
psql "$NEON_DATABASE_URL" <<'SQL'
-- Kill rate per minute, last hour
SELECT date_trunc('minute', finished_at) AS m,
       count(*) FILTER (WHERE prune_reason LIKE '%timeout%')          AS timeouts,
       count(*) FILTER (WHERE prune_reason LIKE '%zero steps%')        AS zero_steps,
       count(*) FILTER (WHERE prune_reason LIKE '%GLIBC%')             AS glibc,
       count(*) FILTER (WHERE prune_reason LIKE '%trainer_kind%')      AS unsupported,
       count(*)                                                        AS total_failed
FROM strategy_queue
WHERE status='failed' AND finished_at > now() - interval '1 hour'
GROUP BY m ORDER BY m DESC;

-- Alive scarabs per account (CORRECT column: railway_acc)
SELECT railway_acc, count(*) FILTER (WHERE last_heartbeat > now() - interval '5 minutes') AS alive,
                         count(*) AS total
FROM scarabs
GROUP BY railway_acc ORDER BY railway_acc;

-- Pending/running FULL81K runs
SELECT canon_name, account, status, steps_budget, created_at, claimed_at, started_at
FROM strategy_queue
WHERE canon_name LIKE 'IGLA-CHAMPION-FULL81K-%' AND status IN ('pending','running')
ORDER BY canon_name;

-- Recent failures with prune_reason
SELECT canon_name, status, prune_reason, finished_at, steps_budget
FROM strategy_queue
WHERE status='failed' AND finished_at > now() - interval '2 hours'
ORDER BY finished_at DESC LIMIT 20;
SQL
```

**Save output to** `/tmp/heal/diag_$(date +%s).txt`.

**Expected patterns:**
- If `timeouts > 0` → worker timeout issue (NOT in DB, check worker code)
- If `zero_steps > 0` → trainer crashing immediately (image/worker issue)
- If multiple accounts have `alive=0` → scarab services down (need restart)

***

## STEP 2 — Check for stale claims (5 sec)

**Note:** This resets "running" experiments that haven't produced data recently.

```bash
psql "$NEON_DATABASE_URL" <<'SQL'
BEGIN;

-- Any claim older than 30 min with no recent bpb_samples (by canon_name+seed) → reset to pending
UPDATE strategy_queue sq
SET status='pending',
    claimed_at=NULL, started_at=NULL,
    last_error='claim expired: scarab died without finishing'
WHERE status='running'
  AND claimed_at < now() - interval '30 minutes'
  AND NOT EXISTS (
    SELECT 1 FROM bpb_samples bs
    WHERE bs.canon_name = sq.canon_name
      AND bs.seed = sq.seed
      AND bs.ts > now() - interval '5 minutes'
  );

SELECT count(*) AS reset_runs FROM strategy_queue WHERE status='pending'
  AND last_error LIKE 'claim expired%';

COMMIT;
SQL
```

***

## STEP 3 — Verify Neon writer flowing (30 sec)

```bash
psql "$NEON_DATABASE_URL" <<'SQL'
-- New bpb_samples in last 5 minutes
SELECT canon_name, step, bpb, ts
FROM bpb_samples
WHERE ts > now() - interval '5 minutes'
ORDER BY ts DESC LIMIT 20;

-- Count writes per minute over last 10 min
SELECT date_trunc('minute', ts) AS m, count(*) AS rows
FROM bpb_samples
WHERE ts > now() - interval '10 minutes'
GROUP BY m ORDER BY m DESC;

-- Summary
SELECT
  count(*) FILTER (WHERE ts > now() - interval '5 minutes') AS rows_last_5min,
  count(DISTINCT canon_name) FILTER (WHERE ts > now() - interval '5 minutes') AS distinct_runs,
  count(*) FILTER (WHERE bpb > 0 AND bpb < 12 AND ts > now() - interval '5 minutes') AS honest_rows
FROM bpb_samples;
SQL
```

**Pass criterion:** `rows_last_5min > 0`. Если 0 → writer broken.

***

## STEP 4 — ⚠️ SKIP: Restart Railway services (API LIMITATION)

**THIS STEP DOES NOT WORK WITH PROJECT-SCOPED TOKENS.**

Project tokens (`RAILWAY_TOKEN_ACC*`) return "Not Authorized" for GraphQL API.

**Options for service restart:**
1. **Manual:** Login to Railway dashboard → navigate to acc2,acc5,acc6,acc7 → restart scarab services
2. **User-level token:** Add `RAILWAY_TOKEN` (user-scoped) to `.env` for GraphQL API access
3. **Railway CLI:** Use `railway login` + `railway link` per project (interactive, not scriptable)

```bash
# Only attempt if user-level RAILWAY_TOKEN exists
if [ -n "$RAILWAY_TOKEN" ]; then
  GRAPHQL_URL="https://backboard.railway.app/graphql/v2"
  # ... GraphQL queries for service restart
else
  echo "⚠️ SKIP: No user-level RAILWAY_TOKEN for GraphQL API access"
fi
```

***

## STEP 5 — Check GATE2/FULL81K health (30 sec)

```bash
psql "$NEON_DATABASE_URL" <<'SQL'
-- GATE2 seeds health check
SELECT canon_name, status, claimed_at, started_at, last_error,
       (SELECT max(step) FROM bpb_samples bs WHERE bs.canon_name = sq.canon_name) AS max_step,
       (SELECT bpb FROM bpb_samples bs WHERE bs.canon_name = sq.canon_name ORDER BY step DESC LIMIT 1) AS last_bpb
FROM strategy_queue sq
WHERE canon_name LIKE 'IGLA-TRAIN_V2-FP32-GATE2-seed%'
ORDER BY canon_name;

-- FULL81K health check
SELECT canon_name, status, claimed_at, started_at, last_error,
       (SELECT max(step) FROM bpb_samples bs WHERE bs.canon_name = sq.canon_name) AS max_step,
       (SELECT bpb FROM bpb_samples bs WHERE bs.canon_name = sq.canon_name ORDER BY step DESC LIMIT 1) AS last_bpb
FROM strategy_queue sq
WHERE canon_name LIKE 'IGLA-CHAMPION-FULL81K-%'
ORDER BY canon_name;
SQL
```

**Issue patterns:**
- `max_step = 1000` stuck → worker crash at checkpoint (check logs)
- `last_error contains "exit status 101"` → image/GLIBC issue
- `last_error contains "timeout"` → worker timeout (check worker code)

***

## STEP 6 — Optional: Trigger Docker image rebuild (if GITHUB_TOKEN set)

If workers are crashing with exit status 101 or GLIBC errors, may need to rebuild image.

```bash
if [ -n "$GITHUB_TOKEN" ]; then
  curl -sX POST "https://api.github.com/repos/gHashTag/trios-trainer-igla/actions/workflows/build-and-push.yml/dispatches" \
    -H "Authorization: Bearer $GITHUB_TOKEN" \
    -H "Accept: application/vnd.github+json" \
    -d '{"ref":"main","inputs":{"force_rebuild":"true","glibc_target":"2.31"}}'

  echo "✅ Image rebuild triggered. Wait ~5 min for build."
else
  echo "⚠️ GITHUB_TOKEN not set, skipping image rebuild"
fi
```

***

## STEP 7 — Write heal-ledger row (10 sec)

**NOTE:** l7_ledger schema: `ts`, `inv_code`, `severity`, `details` (jsonb), `action_taken`, `resolved`

```bash
psql "$NEON_DATABASE_URL" <<SQL
INSERT INTO l7_ledger (ts, inv_code, severity, details, action_taken)
VALUES (
  now(),
  'HEAL_ONESHOT_EXEC',
  'info',
  '{"scarab_health": "TBD", "writer_status": "TBD", "issues_found": "TBD", "recommendation": "TBD"}'::jsonb,
  'HEAL_ONESHOT diagnostic executed, see /tmp/heal/'
);
SQL
```

Replace "TBD" with actual findings from STEP 1-5.

***

## ⚠️ Hard rules (для агента-исполнителя)

1. **Все API tokens — только из `/Users/playra/trios-railway/.env`:**
   - `RAILWAY_TOKEN_ACC0` через `RAILWAY_TOKEN_ACC7` (project-scoped, limited)
   - `RAILWAY_TOKEN` (optional, user-scoped for GraphQL)
   - `NEON_DATABASE_URL`
   - `GITHUB_TOKEN` (optional)

2. **Railway API LIMITATION:** Project tokens do NOT support GraphQL/REST. Cannot restart services programmatically.

3. **All Neon operations — через `psql "$NEON_DATABASE_URL"`**. НЕ через web console.

4. **Schema notes (CRITICAL):**
   - `scarabs.railway_acc` (not `account`)
   - `strategy_queue` has NO `timeout_seconds` column
   - `l7_ledger` uses `details` (jsonb), NOT separate `action/hypothesis/resolution` columns
   - `bpb_samples` has `canon_name` and `seed`, NOT `experiment_id` as FK

5. **При любой ошибке — STOP и записать в `/tmp/heal/error_$(date +%s).txt`.**

6. **Не trigger force-push или PR create.** Только `workflow_dispatch` для image rebuild.

***

## Known Issues (from 2026-05-01 execution)

| Issue | Status | Root Cause |
|-------|--------|------------|
| Trainer checkpoint stall at step 1000 | ⚠️ CONFIRMED | Worker crashes during checkpoint write |
| 4 accounts dead scarabs (acc2,acc5,acc6,acc7) | ❌ Cannot fix via API | Project tokens lack GraphQL access |
| Exit status 101 crashes | ⚠️ Under investigation | Likely image/GLIBC mismatch |
| Writer flowing (limited) | ✅ Working | Alive scarabs producing data |
| Timeout failures | ❌ FALSE hypothesis | No timeout in last 2h — not the issue |

**Resolution required:**
1. Fix worker checkpoint write (step 1000 stall)
2. Manual Railway dashboard intervention OR user-level `RAILWAY_TOKEN` for dead accounts

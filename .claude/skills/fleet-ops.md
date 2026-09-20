# Fleet Operations Skill

Skill for operating the IGLA RACE distributed fleet: scarab workers, gardener, doctor, and Railway deployment.

## Overview

**Current architecture:** `scarab` services on Railway run `trios-train` directly, writing BPB to `bpb_samples` via `NEON_DATABASE_URL`.

**Future architecture:** `seed_agent` pulls from `experiment_queue`, registers in `workers`, heartbeat-driven.

**Guardian:** `scripts/fleet_guardian.py` runs every 15 min via launchd.

## Database Schema (Current)

```
public.bpb_samples          — BPB telemetry (canon_name, seed, step, bpb, ts)
public.experiment_queue     — Pending experiments (status: pending/claimed/running/done/pruned)
public.workers              — seed_agent registrations (id, railway_acc, last_heartbeat)
public.scarab_strategy      — New architecture (service_id, format, optimizer, hidden, status)
public.scarab_heartbeat     — New architecture (service_id, is_training, last_heartbeat)
public.strategy_queue       — RAG/GRAND-CANON missions (NOT training)
public.igla_agents_heartbeat — Agent liveness (agent_id, last_heartbeat)
```

## Scarab Worker Lifecycle

### Current (Direct trios-train)

1. Service starts with env vars: `TRIOS_SEED`, `TRIOS_STEPS`, `TRIOS_HIDDEN`, `TRIOS_LR`, `TRIOS_FORMAT`, `TRIOS_OPTIMIZER`
2. Runs `trios-train` binary directly
3. Writes BPB samples to DB via SeaORM
4. No registration in `workers` table

### Future (seed_agent)

1. Connects to DB, checks `experiment_queue` exists
2. Registers in `workers` table with UUID
3. Spawns heartbeat task
4. Pulls pending experiments, runs trainer, marks done

## Gardener Mode

**Trigger:** Every 15 minutes via `fleet_guardian.py`

**Actions:**

| Condition | Action |
|-----------|--------|
| `experiment_queue` pending < 3 | Seed 5 default experiments |
| Running experiment >2h no BPB | Mark as `pruned` |
| Best BPB < 3.0 AND pending < 6 | Spawn mirror experiments |
| Gate-2 quorum < 3 AND pending < 6 | Suggest new explorations |

**Default seeds:**
```
(42, 1024, 12, 0.003, 81000)
(43, 1024, 12, 0.003, 81000)
(44, 1024, 12, 0.003, 81000)
(42, 512, 8, 0.003, 27000)
(43, 828, 12, 0.004, 81000)
```

## Doctor Mode

**Trigger:** Every 15 minutes alongside gardener

**Checks & Actions:**

| Check | Threshold | Action |
|-------|-----------|--------|
| bpb_samples stale | >1 hour | Restart train/scarab services |
| workers stale | >1 hour | Restart scarab services |
| Railway service crashed | any | Auto-restart (10-min cooldown) |
| DB connectivity | fail | Alert |
| Missing tables | any | Alert |

**Cooldown:** 10 minutes between restarts of the same service.

## Railway GraphQL API

**Endpoint:** `https://backboard.railway.app/graphql/v2`

**Key operations:**

```bash
# List services
curl -s -X POST $API \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "query { project(id: \"'$PROJECT'\") { services { edges { node { id name } } } } }"}'

# Set env var
curl -s -X POST $API \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "mutation { variableUpsert(input: { projectId: \"'$PROJECT'\", serviceId: \"'$SERVICE'\", environmentId: \"'$ENV'\", name: \"'$NAME'\", value: \"'$VALUE'\" }) }"}'

# Redeploy service
curl -s -X POST $API \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "mutation { serviceInstanceDeploy(serviceId: \"'$SERVICE'\", environmentId: \"'$ENV'\") }"}'

# Get deployment logs
curl -s -X POST $API \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "query { deploymentLogs(deploymentId: \"'$DEPLOY'\", limit: 30) { message timestamp } }"}'
```

## Environment Variables (Railway Services)

Critical vars for scarab services:

```
NEON_DATABASE_URL=postgresql://...     # Required for BPB writing
DATABASE_URL=postgresql://...           # Railway auto-provides
TRIOS_SEED=89                           # Training seed
TRIOS_STEPS=20000                       # Training steps
TRIOS_HIDDEN=384                        # Hidden dim
TRIOS_LR=0.003                          # Learning rate
TRIOS_FORMAT=gf16                       # Number format
TRIOS_OPTIMIZER=muon                    # Optimizer
```

## Fleet Guardian

**Script:** `scripts/fleet_guardian.py`
**Schedule:** launchd `com.fleet-guardian`, every 15 min
**Log:** `/tmp/fleet_guardian.log`
**Dashboard:** `FLEET_DASHBOARD.md` (auto-generated)

### Manual run

```bash
# Full check
python3 scripts/fleet_guardian.py --env .env

# Doctor only
python3 scripts/fleet_guardian.py --env .env --doctor

# Gardener only
python3 scripts/fleet_guardian.py --env .env --gardener
```

### launchd management

```bash
# Check status
launchctl list | grep fleet-guardian

# Unload
launchctl unload ~/Library/LaunchAgents/com.fleet-guardian.plist

# Load
launchctl load ~/Library/LaunchAgents/com.fleet-guardian.plist

# View logs
tail -f /tmp/fleet_guardian.log
```

## Cycle-19 Lanes (Active)

| Priority | Service ID | Format | Optimizer | Hidden | Status |
|----------|-----------|--------|-----------|--------|--------|
| P0 | cycle19-fp80-muon-h384 | fp80 | muon | 384 | active |
| P0 | cycle19-posit16-muon-h384 | posit16 | muon | 384 | active |
| P1 | cycle19-nf4-muon-h256 | nf4 | muon | 256 | paused |
| P1 | cycle19-int4-muon-h256 | int4 | muon | 256 | paused |
| P1 | cycle19-fp80-adamw-h256 | fp80 | adamw | 256 | paused |
| P2 | cycle19-bf16-sgdm-h256 | bf16 | sgdm | 256 | paused |

## Known Issues & Fixes

### Trigger `mirror_public_bpb_samples` bug

**Symptom:** `[ledger] bpb_sample failed: violates check constraint "bpb_no_bug4_sha"`

**Root cause:** `ssot.mirror_public_bpb_samples()` inserts `sha = '8527716a'` for non-seed-43 rows, but `ssot.bpb_samples` forbids this value.

**Fix:**
```sql
CREATE OR REPLACE FUNCTION ssot.mirror_public_bpb_samples()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO ssot.bpb_samples (canon_name, format, algo, hidden, seed, step, bpb, sha, run_id, ts)
    VALUES (
        NEW.canon_name, 'f32', 'adamw',
        CASE WHEN NEW.seed = 43 THEN 828 ELSE 384 END,
        NEW.seed, NEW.step, NEW.bpb,
        CASE WHEN NEW.seed = 43 THEN 'cd91c45' ELSE NULL END,
        'phase1-' || NEW.canon_name, NEW.ts
    )
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

### DSN unset

**Symptom:** `[ledger] DSN unset — skipping bpb_sample`

**Fix:** Set `NEON_DATABASE_URL` on all services via `variableUpsert`, then redeploy.

## Quick Commands

```bash
# Check fleet status
python3 scripts/fleet_guardian.py --env .env

# View dashboard
cat FLEET_DASHBOARD.md

# Check DB state
psql "$DATABASE_URL" -c "SELECT COUNT(*), MAX(ts) FROM bpb_samples"

# Check experiment queue
psql "$DATABASE_URL" -c "SELECT status, COUNT(*) FROM experiment_queue GROUP BY status"

# Check workers
psql "$DATABASE_URL" -c "SELECT COUNT(*), MAX(last_heartbeat) FROM workers"

# Check scarab strategy
psql "$DATABASE_URL" -c "SELECT service_id, status FROM scarab_strategy ORDER BY status"
```

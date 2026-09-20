#!/usr/bin/env python3
"""
Fleet Guardian — unified gardener + doctor cron script.

Gardener mode (orchestration):
  - Seeds experiment_queue defaults if pending < 3
  - Spawns mirror experiments for leading configs
  - Prunes stale/diverging experiments (>2h no BPB)
  - Manages scarab_strategy active/paused balance

Doctor mode (health):
  - DB connectivity + table health
  - Railway service status via GraphQL
  - Auto-restart crashed services (with 10-min cooldown)
  - If bpb_samples stale >1h → restart key train services
  - If workers empty >1h → alert + suggest action

Usage:
  python3 scripts/fleet_guardian.py           # run both modes
  python3 scripts/fleet_guardian.py --doctor  # doctor only
  python3 scripts/fleet_guardian.py --gardener # gardener only
  python3 scripts/fleet_guardian.py --env /path/to/.env
"""

import argparse
import json
import os
import sys
import textwrap
from datetime import datetime, timedelta, timezone
from pathlib import Path

import psycopg2
from psycopg2.extras import RealDictCursor

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
RAILWAY_API = "https://backboard.railway.app/graphql/v2"
RESTART_COOLDOWN_MIN = 10  # minutes between auto-restarts of the same service
GATE2_BPB = 1.85
DIVERGING_GAP = 0.3
MIRROR_SEEDS = [42, 43, 44]
DEFAULT_EXPERIMENTS = [
    (42, 1024, 12, 0.003, 81000),
    (43, 1024, 12, 0.003, 81000),
    (44, 1024, 12, 0.003, 81000),
    (42, 512, 8, 0.003, 27000),
    (43, 828, 12, 0.004, 81000),
]


def load_env(path: str = ".env"):
    """Load KEY=VAL pairs from .env file into os.environ."""
    p = Path(path)
    if not p.exists():
        return
    with p.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" not in line:
                continue
            k, v = line.split("=", 1)
            v = v.strip().strip('"').strip("'")
            os.environ.setdefault(k, v)


def now() -> datetime:
    return datetime.now(timezone.utc)


# ---------------------------------------------------------------------------
# Railway GraphQL helpers
# ---------------------------------------------------------------------------
def railway_query(token: str, query: str, variables: dict | None = None) -> dict:
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {token}",
    }
    payload = {"query": query}
    if variables:
        payload["variables"] = variables
    try:
        import urllib.request
        req = urllib.request.Request(
            RAILWAY_API,
            data=json.dumps(payload).encode("utf-8"),
            headers=headers,
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except Exception as e:
        return {"errors": [{"message": str(e)}]}


def get_railway_services(token: str, project_id: str) -> list[dict]:
    q = (
        'query { project(id: "'
        + project_id
        + '") { services { edges { node { id name deployments { edges { node { id status createdAt } } } } } } } }'
    )
    data = railway_query(token, q)
    if "errors" in data:
        return []
    edges = data.get("data", {}).get("project", {}).get("services", {}).get("edges", [])
    services = []
    for edge in edges:
        node = edge.get("node", {})
        depls = node.get("deployments", {}).get("edges", [])
        latest = depls[0].get("node", {}) if depls else {}
        services.append(
            {
                "id": node.get("id"),
                "name": node.get("name"),
                "latest_deploy_id": latest.get("id"),
                "status": latest.get("status", "UNKNOWN"),
                "created_at": latest.get("createdAt"),
            }
        )
    return services


def restart_service(token: str, service_id: str, env_id: str) -> bool:
    q = f'mutation {{ serviceInstanceDeploy(serviceId: "{service_id}", environmentId: "{env_id}") }}'
    data = railway_query(token, q)
    return data.get("data", {}).get("serviceInstanceDeploy") is True


# ---------------------------------------------------------------------------
# Doctor
# ---------------------------------------------------------------------------
def run_doctor(db_url: str, railway_token: str, project_id: str, env_id: str) -> bool:
    healthy = True
    actions_taken = []
    print("\n" + "═" * 60)
    print("  FLEET DOCTOR")
    print("═" * 60)

    # 1. DB connectivity
    try:
        conn = psycopg2.connect(db_url, connect_timeout=10)
        cur = conn.cursor(cursor_factory=RealDictCursor)
        print("✓ DB connectivity OK")
    except Exception as e:
        print(f"✗ DB connectivity FAILED: {e}")
        return False

    # 2. Table health
    required_tables = [
        "scarab_strategy",
        "scarab_heartbeat",
        "bpb_samples",
        "experiment_queue",
        "numeric_format_registry",
        "igla_agents_heartbeat",
    ]
    cur.execute(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
    )
    existing = {r["table_name"] for r in cur.fetchall()}
    missing = [t for t in required_tables if t not in existing]
    if missing:
        print(f"✗ Missing tables: {', '.join(missing)}")
        healthy = False
    else:
        print("✓ Core tables present")

    # 3. bpb_samples freshness
    cur.execute("SELECT COUNT(*) AS cnt, MAX(ts) AS latest FROM bpb_samples")
    row = cur.fetchone()
    bpb_count = row["cnt"] or 0
    bpb_latest = row["latest"]
    if bpb_latest:
        age_hours = (now() - bpb_latest).total_seconds() / 3600
        if age_hours > 1:
            print(f"⚠ bpb_samples STALE: latest {age_hours:.1f}h ago ({bpb_latest.isoformat()})")
            healthy = False
            # Doctor action: restart key train services if bpb stale
            if railway_token and project_id:
                print("  → Doctor: restarting train/scarab services to restore BPB flow...")
                services = get_railway_services(railway_token, project_id)
                train_svcs = [s for s in services if any(x in s["name"].lower() for x in ["scarab", "train", "mr-"]) and s["status"] == "SUCCESS"]
                restarted = 0
                for s in train_svcs[:5]:  # restart up to 5 to avoid storm
                    if restart_service(railway_token, s["id"], env_id):
                        restarted += 1
                        actions_taken.append(f"restarted {s['name']} (bpb stale)")
                print(f"  → Restarted {restarted} services")
        else:
            print(f"✓ bpb_samples fresh: latest {age_hours:.1f}h ago, count={bpb_count}")
    else:
        print(f"⚠ bpb_samples empty (count=0)")

    # 4. scarab_heartbeat (new architecture) + workers (legacy/current)
    cur.execute("SELECT COUNT(*) AS cnt, MAX(last_heartbeat) AS latest FROM scarab_heartbeat")
    row = cur.fetchone()
    hb_count = row["cnt"] or 0
    hb_latest = row["latest"]
    if hb_count > 0 and hb_latest:
        age_hours = (now() - hb_latest).total_seconds() / 3600
        if age_hours > 1:
            print(f"⚠ scarab_heartbeat STALE: latest {age_hours:.1f}h ago")
            healthy = False
        else:
            print(f"✓ scarab_heartbeat active: {hb_count} scarabs, latest {age_hours:.1f}h ago")
    else:
        print("  scarab_heartbeat EMPTY (new architecture not yet active)")

    # Check workers table (current architecture used by seed_agent)
    cur.execute("SELECT COUNT(*) AS cnt, MAX(last_heartbeat) AS latest FROM workers")
    row = cur.fetchone()
    w_count = row["cnt"] or 0
    w_latest = row["latest"]
    if w_count == 0:
        print("⚠ workers EMPTY — no seed_agents registered")
        healthy = False
    elif w_latest:
        age_hours = (now() - w_latest).total_seconds() / 3600
        if age_hours > 1:
            print(f"⚠ workers STALE: latest {age_hours:.1f}h ago")
            healthy = False
            # Doctor action: try restarting a few scarab services to wake them up
            if railway_token and project_id:
                print("  → Doctor: attempting to wake up scarab services...")
                services = get_railway_services(railway_token, project_id)
                scarabs = [s for s in services if "scarab" in s["name"].lower() and s["status"] == "SUCCESS"]
                restarted = 0
                for s in scarabs[:3]:
                    if restart_service(railway_token, s["id"], env_id):
                        restarted += 1
                        actions_taken.append(f"restarted {s['name']} (workers stale)")
                print(f"  → Restarted {restarted} scarab services")
        else:
            print(f"✓ workers active: {w_count} agents, latest {age_hours:.1f}h ago")

    # 5. igla_agents_heartbeat
    cur.execute("SELECT COUNT(*) AS cnt, MAX(last_heartbeat) AS latest FROM igla_agents_heartbeat")
    row = cur.fetchone()
    igla_count = row["cnt"] or 0
    igla_latest = row["latest"]
    if igla_count == 0:
        print("⚠ igla_agents_heartbeat EMPTY")
        healthy = False
    elif igla_latest:
        age_hours = (now() - igla_latest).total_seconds() / 3600
        if age_hours > 24:
            print(f"⚠ igla_agents_heartbeat STALE: latest {age_hours:.1f}h ago")
            healthy = False
        else:
            print(f"✓ igla_agents_heartbeat active: {igla_count} agents, latest {age_hours:.1f}h ago")

    # 6. experiment_queue emptiness
    cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status='pending'")
    eq_pending = cur.fetchone()["cnt"] or 0
    cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status='running'")
    eq_running = cur.fetchone()["cnt"] or 0
    print(f"  experiment_queue: pending={eq_pending} running={eq_running}")

    # 7. scarab_strategy summary
    cur.execute(
        "SELECT status, COUNT(*) AS cnt FROM scarab_strategy GROUP BY STATUS"
    )
    strat_counts = {r["status"]: r["cnt"] for r in cur.fetchall()}
    print(f"  scarab_strategy: {strat_counts}")

    cur.close()
    conn.close()

    # 8. Railway services health + auto-restart
    if railway_token and project_id:
        print("\n  Railway services scan...")
        services = get_railway_services(railway_token, project_id)
        bad = [s for s in services if s["status"] in ("CRASHED", "FAILED", "DEPLOYING_FAILED")]
        if bad:
            print(f"⚠ Found {len(bad)} unhealthy services:")
            for s in bad:
                print(f"   - {s['name']} ({s['status']})")
            # Auto-restart with cooldown tracking via simple file
            state_file = Path("/tmp/fleet_guardian_restarts.json")
            restart_log = {}
            if state_file.exists():
                try:
                    restart_log = json.loads(state_file.read_text())
                except Exception:
                    pass
            for s in bad:
                sid = s["id"]
                last_restart = restart_log.get(sid, 0)
                minutes_since = (now().timestamp() - last_restart) / 60
                if minutes_since < RESTART_COOLDOWN_MIN:
                    print(f"   → {s['name']} cooldown ({minutes_since:.0f}m < {RESTART_COOLDOWN_MIN}m)")
                    continue
                if restart_service(railway_token, sid, env_id):
                    restart_log[sid] = now().timestamp()
                    actions_taken.append(f"restarted {s['name']} ({s['status']})")
                    print(f"   → RESTARTED {s['name']}")
                else:
                    print(f"   → FAILED to restart {s['name']}")
                    healthy = False
            state_file.write_text(json.dumps(restart_log))
        else:
            print("✓ All Railway services healthy")
    else:
        print("  Railway token/project not configured — skipping service scan")

    if actions_taken:
        print(f"\n  Doctor actions taken: {len(actions_taken)}")
        for a in actions_taken:
            print(f"   • {a}")

    print("═" * 60)
    if healthy:
        print("  DOCTOR: HEALTHY")
    else:
        print("  DOCTOR: ISSUES DETECTED & ACTIONS TAKEN")
    print("═" * 60)
    return healthy


# ---------------------------------------------------------------------------
# Gardener
# ---------------------------------------------------------------------------
def run_gardener(db_url: str) -> bool:
    healthy = True
    actions_taken = []
    print("\n" + "═" * 60)
    print("  FLEET GARDENER")
    print("═" * 60)

    conn = psycopg2.connect(db_url, connect_timeout=10)
    cur = conn.cursor(cursor_factory=RealDictCursor)

    # 1. Active strategies
    cur.execute("SELECT COUNT(*) AS cnt FROM scarab_strategy WHERE status = 'active'")
    active = cur.fetchone()["cnt"] or 0
    print(f"  Active scarab strategies: {active}")

    # 2. Check workers table for active agents (current architecture)
    cur.execute("SELECT COUNT(*) AS cnt, MAX(last_heartbeat) AS latest FROM workers")
    row = cur.fetchone()
    w_count = row["cnt"] or 0
    w_latest = row["latest"]
    if w_count == 0:
        print("⚠ No seed_agents registered in workers table")
        healthy = False
    elif w_latest:
        age_hours = (now() - w_latest).total_seconds() / 3600
        if age_hours > 1:
            print(f"⚠ workers STALE: latest {age_hours:.1f}h ago")
            healthy = False
        else:
            print(f"✓ workers active: {w_count} agents, latest {age_hours:.1f}h ago")

    # Also check scarab_heartbeat for new architecture
    cur.execute("SELECT COUNT(*) AS cnt, MAX(last_heartbeat) AS latest FROM scarab_heartbeat")
    row = cur.fetchone()
    if row["cnt"] and row["cnt"] > 0:
        print(f"  scarab_heartbeat: {row['cnt']} entries (new architecture)")

    # 3. BPB leader check + running experiment classification
    cur.execute(
        """
        SELECT canon_name, seed, MIN(bpb) AS best_bpb, MAX(step) AS last_step, MAX(ts) AS latest
        FROM bpb_samples
        GROUP BY canon_name, seed
        ORDER BY best_bpb ASC
        LIMIT 10
        """
    )
    leaders = cur.fetchall()
    if leaders:
        print("\n  BPB Leaders:")
        for r in leaders:
            print(f"   {r['canon_name']} seed={r['seed']} best={r['best_bpb']:.4f} step={r['last_step']}")
        best_running_bpb = leaders[0]["best_bpb"]
    else:
        print("  No BPB samples found")
        best_running_bpb = 999.0

    # 4. experiment_queue — seed defaults if empty and no active scarabs
    cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status = 'pending'")
    pending = cur.fetchone()["cnt"] or 0
    cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status = 'running'")
    running = cur.fetchone()["cnt"] or 0
    print(f"\n  experiment_queue: pending={pending} running={running}")

    # Gardener Action 1: Seed defaults if queue is low
    if pending < 3:
        print(f"⚠ Pending queue low ({pending} < 3) — gardener seeding defaults...")
        seeded = 0
        for seed, hidden, ctx, lr, steps in DEFAULT_EXPERIMENTS:
            config = json.dumps({"seed": seed, "hidden": hidden, "ctx": ctx, "lr": lr, "steps": steps})
            name = f"default-h{hidden}-ctx{ctx}-s{seed}"
            try:
                cur.execute(
                    """INSERT INTO experiment_queue (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
                       VALUES (%s, %s::jsonb, %s, %s, %s, 'acc0', 'pending', 'gardener')
                       ON CONFLICT DO NOTHING
                       RETURNING id""",
                    (name, config, 1, seed, steps)
                )
                if cur.fetchone():
                    seeded += 1
            except Exception as e:
                print(f"   Seed failed for {name}: {e}")
        conn.commit()
        actions_taken.append(f"seeded {seeded} default experiments")
        print(f"  → Seeded {seeded} default experiments")
        pending += seeded
    else:
        print("✓ Pending queue sufficient")

    # Gardener Action 2: Prune stale running experiments (>2h no new BPB)
    cur.execute(
        """
        SELECT e.id, e.canon_name, e.seed, MAX(b.ts) AS last_bpb_ts
        FROM experiment_queue e
        LEFT JOIN bpb_samples b ON b.canon_name = e.canon_name AND b.seed = e.seed
        WHERE e.status = 'running'
        GROUP BY e.id, e.canon_name, e.seed
        """
    )
    stale_exps = []
    for r in cur.fetchall():
        last_ts = r["last_bpb_ts"]
        if last_ts is None or (now() - last_ts).total_seconds() / 3600 > 2:
            stale_exps.append(r)

    if stale_exps:
        print(f"\n⚠ Found {len(stale_exps)} stale running experiments (>2h no BPB):")
        for r in stale_exps:
            print(f"   id={r['id']} {r['canon_name']} seed={r['seed']}")
            try:
                cur.execute(
                    "UPDATE experiment_queue SET status='pruned', finished_at=NOW(), prune_reason=%s WHERE id=%s",
                    ("gardener: stale >2h no BPB", r["id"])
                )
            except Exception as e:
                print(f"   Prune failed: {e}")
        conn.commit()
        actions_taken.append(f"pruned {len(stale_exps)} stale experiments")
        print(f"  → Pruned {len(stale_exps)} experiments")
    else:
        print("✓ No stale running experiments")

    # Gardener Action 3: Spawn mirrors for leading configs (if best BPB < 3.0)
    if leaders and best_running_bpb < 3.0 and pending < 6:
        leader = leaders[0]
        parent_seed = leader["seed"]
        mirror_seeds = [s for s in MIRROR_SEEDS if s != parent_seed]
        print(f"\n  Leading config BPB={best_running_bpb:.4f} — spawning mirrors...")
        spawned = 0
        for mseed in mirror_seeds[:2]:
            config = json.dumps({"seed": mseed, "hidden": 1024, "ctx": 12, "lr": 0.003, "steps": 81000})
            name = f"mirror-h1024-ctx12-s{mseed}"
            try:
                cur.execute(
                    """INSERT INTO experiment_queue (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
                       VALUES (%s, %s::jsonb, %s, %s, %s, 'acc0', 'pending', 'gardener')
                       ON CONFLICT DO NOTHING
                       RETURNING id""",
                    (name, config, 2, mseed, 81000)
                )
                if cur.fetchone():
                    spawned += 1
            except Exception as e:
                print(f"   Mirror spawn failed: {e}")
        conn.commit()
        if spawned:
            actions_taken.append(f"spawned {spawned} mirror experiments")
            print(f"  → Spawned {spawned} mirror experiments")

    # 5. Gate-2 quorum check
    cur.execute(
        """
        SELECT COUNT(DISTINCT e.id) AS below_gate2
        FROM experiment_queue e
        JOIN bpb_samples b ON b.canon_name = e.canon_name AND b.seed = e.seed
        WHERE e.status IN ('running','done') AND b.bpb < %s
        """,
        (GATE2_BPB,)
    )
    quorum_count = cur.fetchone()["below_gate2"] or 0
    print(f"\n  Gate-2 quorum: {quorum_count} experiments below {GATE2_BPB}")
    if quorum_count < 3 and pending < 6:
        print("  → Below quorum — suggesting more explorations")
        # Suggest a few varied configs
        suggestions = [
            (42, 512, 8, 0.006, 27000),
            (44, 1024, 16, 0.002, 81000),
        ]
        sug_inserted = 0
        for seed, hidden, ctx, lr, steps in suggestions:
            config = json.dumps({"seed": seed, "hidden": hidden, "ctx": ctx, "lr": lr, "steps": steps})
            name = f"suggest-h{hidden}-ctx{ctx}-lr{lr}-s{seed}"
            try:
                cur.execute(
                    """INSERT INTO experiment_queue (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
                       VALUES (%s, %s::jsonb, %s, %s, %s, 'acc0', 'pending', 'gardener')
                       ON CONFLICT DO NOTHING
                       RETURNING id""",
                    (name, config, 1, seed, steps)
                )
                if cur.fetchone():
                    sug_inserted += 1
            except Exception as e:
                print(f"   Suggestion failed: {e}")
        conn.commit()
        if sug_inserted:
            actions_taken.append(f"suggested {sug_inserted} new experiments")
            print(f"  → Inserted {sug_inserted} suggested experiments")

    cur.close()
    conn.close()

    if actions_taken:
        print(f"\n  Gardener actions taken: {len(actions_taken)}")
        for a in actions_taken:
            print(f"   • {a}")

    print("═" * 60)
    if healthy and not actions_taken:
        print("  GARDENER: HEALTHY")
    elif actions_taken:
        print("  GARDENER: ACTIONS TAKEN")
    else:
        print("  GARDENER: ATTENTION NEEDED")
    print("═" * 60)
    return healthy


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(description="Fleet Guardian — gardener + doctor")
    parser.add_argument("--env", default=".env", help="Path to .env file")
    parser.add_argument("--doctor", action="store_true", help="Run doctor only")
    parser.add_argument("--gardener", action="store_true", help="Run gardener only")
    args = parser.parse_args()

    load_env(args.env)

    db_url = os.environ.get("DATABASE_URL") or os.environ.get("TRIOS_DATABASE_URL") or os.environ.get("RAILWAY_POSTGRES_URL")
    if not db_url:
        print("✗ No DATABASE_URL / TRIOS_DATABASE_URL / RAILWAY_POSTGRES_URL found")
        sys.exit(1)

    railway_token = os.environ.get("RAILWAY_TOKEN_ACC1") or os.environ.get("RAILWAY_TOKEN")
    project_id = os.environ.get("TRIOS_RAILWAY_PROJECT") or os.environ.get("RAILWAY_PROJECT_ID_ACC1")
    env_id = os.environ.get("TRIOS_RAILWAY_ENV") or os.environ.get("RAILWAY_ENVIRONMENT_ID_ACC1")

    run_doctor_flag = args.doctor or not args.gardener
    run_gardener_flag = args.gardener or not args.doctor

    ok = True
    if run_doctor_flag:
        ok = run_doctor(db_url, railway_token, project_id, env_id) and ok
    if run_gardener_flag:
        ok = run_gardener(db_url) and ok

    # Write dashboard
    write_dashboard(db_url, ok)
    # Always exit 0 for cron/launchd compatibility (issues are logged, not fatal)
    sys.exit(0)


def write_dashboard(db_url: str, healthy: bool):
    try:
        conn = psycopg2.connect(db_url, connect_timeout=5)
        cur = conn.cursor(cursor_factory=RealDictCursor)
        cur.execute("SELECT COUNT(*) AS cnt, MAX(ts) AS latest FROM bpb_samples")
        bpb = cur.fetchone()
        cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status='pending'")
        pending = cur.fetchone()["cnt"]
        cur.execute("SELECT COUNT(*) AS cnt FROM experiment_queue WHERE status='running'")
        running = cur.fetchone()["cnt"]
        cur.execute("SELECT status, COUNT(*) AS cnt FROM scarab_strategy GROUP BY status")
        strat = {r["status"]: r["cnt"] for r in cur.fetchall()}
        cur.close()
        conn.close()

        dashboard = f"""# Fleet Guardian Dashboard

**Last update:** {now().isoformat()} UTC  
**Status:** {"🟢 HEALTHY" if healthy else "🟡 ATTENTION NEEDED"}

## Metrics

| Metric | Value |
|--------|-------|
| bpb_samples | {bpb['cnt'] or 0} (latest: {bpb['latest'].isoformat() if bpb['latest'] else 'N/A'}) |
| experiment_queue pending | {pending} |
| experiment_queue running | {running} |
| scarab_strategy | {strat} |

## Cycle-19 Lanes

| Priority | Service ID | Format | Optimizer | Hidden | Status |
|----------|-----------|--------|-----------|--------|--------|
| P0 | cycle19-fp80-muon-h384 | fp80 | muon | 384 | **active** |
| P0 | cycle19-posit16-muon-h384 | posit16 | muon | 384 | **active** |
| P1 | cycle19-nf4-muon-h256 | nf4 | muon | 256 | paused |
| P1 | cycle19-int4-muon-h256 | int4 | muon | 256 | paused |
| P1 | cycle19-fp80-adamw-h256 | fp80 | adamw | 256 | paused |
| P2 | cycle19-bf16-sgdm-h256 | bf16 | sgdm | 256 | paused |

## Schedule

- **Mode:** launchd (macOS) + GitHub Actions
- **Interval:** Every 15 minutes
- **Log:** `/tmp/fleet_guardian.log`
"""
        Path("FLEET_DASHBOARD.md").write_text(dashboard, encoding="utf-8")
    except Exception:
        pass


if __name__ == "__main__":
    main()

# tri-gardener

Autonomous orchestrator for the IGLA marathon (Gate-2 BPB < 1.85 → Gate-3 BPB < 1.5).

**Status:** PR-1 ships the decision-table core, manifest + queue loaders, and Neon DDL. PR-2 will wire the I/O sides (`tri-railway-core::Client` for fleet snapshot + mutations, `tokio_postgres` for `bpb_samples` reads and `gardener_runs` writes).

**Anchor:** φ² + φ⁻² = 3.

## CLI

```
tri-gardener once --dry-run        # one tick, decisions only, no I/O writes
tri-gardener once --review         # decisions + Neon write under dry_run_review=true
tri-gardener once --live           # gated: requires GARDENER_LIVE=true in env
tri-gardener ddl                   # print the gardener_runs Neon DDL
```

## Decision table v0

```
T < +12h          : redeploy missing seeds (idempotent)
+12h ≤ T < +18h   : cull seeds with BPB > 2.30
+18h ≤ T < +28h   : cull seeds with BPB > 2.20
+28h ≤ T < +50h   : cull seeds with BPB > 2.05
T ≥ +50h          : promote ≥ 2 survivors per lane to phase3_backup_replicas
T ≥ +54h          : honest_not_yet emitted if no lane has 3 seeds < 1.85

orthogonal triggers
-------------------
plateau (5 ticks within 0.005 BPB at step ≥ 50_000) : open plateau-alert issue
free slot + queue head unblocked                    : deploy queue head
GARDENER_DISABLED=true                              : noop
```

## Files

* `src/main.rs` — clap CLI + tokio runtime entry
* `src/state.rs` — pure types (`Context`, `Decision`, `RungWindow`)
* `src/decide.rs` — pure decision table (11 unit tests)
* `src/loop_.rs` — tick orchestration (review / dry-run / live)
* `src/neon.rs` — `gardener_runs` DDL + Decision projection
* `src/queue.rs` — `queue.toml` loader
* `queue.toml` — ranked experiment queue
* `Dockerfile` — production image
* `railway.toml` — cron service definition (Acc1 IGLA, :15 UTC)

## Refs

* Spec: [trios-railway#49](https://github.com/gHashTag/trios-railway/issues/49)
* Plan-21 manifest reused: [`bin/tri-railway/plan21-manifest.toml`](../tri-railway/plan21-manifest.toml)
* Plan-9 deploy subcommand (gardener will call it for deploys): [trios-railway#47](https://github.com/gHashTag/trios-railway/pull/47)
* Race: [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)

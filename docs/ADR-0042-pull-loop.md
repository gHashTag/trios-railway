# ADR-0042 — Sovereign Scarab v4 Pull-Loop (SSOT invariant)

Status: **ACCEPTED** (binding, 2026-05-17)
Anchor: `phi^2 + phi^-2 = 3`

## Invariant (hard)

Scarabs (matrix-runner / trainer / strategy services) are **always live on Railway**:
deploy once, forever. Their control plane is the Postgres SSOT table
`ssot.scarab_strategy`, not Railway's GraphQL API.

Every scarab implements a pull loop:

```
loop {
    row = SELECT * FROM ssot.scarab_strategy WHERE service_id = $me;
    apply(row.optimizer, row.format, row.hidden, row.lr, row.seed,
          row.steps, row.status='active');
    if (row changed since last tick) graceful_self_restart();
    write_bpb_sample_to ssot.bpb_samples;
    sleep N seconds;
}
```

Control is exercised by Queen Hive (the operator/MCP) via `INSERT`/`UPDATE`
into `ssot.scarab_strategy` through the writer MCP. The fleet converges in
the next tick (typically ≤30 s) without any Railway-side action.

## What is FORBIDDEN for scarab control

The following Railway GraphQL operations **must not** be issued against the
scarab fleet from this repo:

- `variableUpsert` — env mutation is push-control; SSOT pull supersedes it.
- `serviceInstanceUpdate` — image/source pin is template-only (DR restore),
  not a steady-state control knob.
- `serviceInstanceDeployV2` / `serviceInstanceRedeploy` — scarabs self-restart
  on `ssot.scarab_strategy` row change; redeploy is never required for
  strategy changes.
- `serviceDelete` — scarabs are forever; quarantine via SSOT `status` column,
  do not delete.

Symbolic name for the legacy push surface: **`LEGACY_PUSH_PATH_DISABLED`**.

## Token policies (defensive)

- No new code path may read `RAILWAY_API_TOKEN`. Use the single
  `RAILWAY_TOKEN` (or per-account `RAILWAY_TOKEN_ACC{N}`) only for the two
  remaining authorized surfaces (see below).
- `Authorization: Bearer` vs `Project-Access-Token` is auto-detected by
  `crates/trios-railway-core/src/transport.rs`; do not hand-roll headers
  in new code.

## What is still PERMITTED

The Rust crate `trios-railway-core` and the `tri-railway` binary expose
Railway GraphQL because **two non-scarab surfaces** legitimately need it:

1. **Read-only diagnostics** — `tri-railway service list`, audit watchdog,
   fleet snapshot, MCP diagnose. These cannot violate the invariant.
2. **MCP control-plane recovery** — `mcp-emergency-redeploy.yml` and
   `writer-env-fix.yml` mutate the trios-railway-mcp / writer sidecar
   services themselves, which are operator-tier infrastructure, not
   scarabs. They are kept manual + `confirm == 'PHI'` guarded and are out
   of scope of this ADR.
3. **Disaster Recovery** — full-fleet recreation from
   `disaster-recovery/fleet-snapshot.json` after an account ban.
   Documented in `docs/DISASTER_RECOVERY.md` and `deploy-from-template.yml`.
   DR exists precisely so that the steady-state pull loop can assume
   "scarabs are forever".

## Compatibility shims (kept for git history)

Workflows that previously pushed env / redeploys to the scarab fleet are
gated on `if: false` and emit an `::error::` pointing to this ADR when
dispatched. They are kept (not deleted) so that historical CI runs remain
linkable.

The Rust mutation functions in `crates/trios-railway-core/src/mutations.rs`
are still compiled and tested, but the operator-facing CLI verbs
(`tri-railway service deploy / redeploy / delete`) check for
`LEGACY_PUSH_PATH_ENABLE=1` and refuse otherwise. CI / cron must never
set that env var.

## Closed by this ADR

- gHashTag/trios-railway#43, #114, #116 — old Railway MCP routes
  (`railway_service_deploy / redeploy / delete`): keep handler, refuse at
  runtime unless `LEGACY_PUSH_PATH_ENABLE=1` is set explicitly by an
  operator.
- gHashTag/trios-railway#126 — MCP telemetry env upsert + redeploy:
  retained as `mcp-emergency-redeploy.yml` / `writer-env-fix.yml` because
  they target the MCP / writer service (non-scarab), not the fleet.
- gHashTag/trios-railway#137 — watchdog `Authorization` header: addressed
  by `transport.rs` auto-detection; no scarab control path involved.

## Verification

```
grep -rn 'variableUpsert\|serviceInstanceDeployV2\|serviceInstanceRedeploy\|serviceDelete' \
    .github/workflows/ \
  | grep -v 'L-SS7\|LEGACY_PUSH_PATH_DISABLED\|if: false\|deprecated'
```

Must return only the four allowlisted entry points:
`mcp-emergency-redeploy.yml`, `writer-env-fix.yml`, `deploy-from-template.yml`,
and the Rust core/MCP tool definitions guarded by `LEGACY_PUSH_PATH_ENABLE`.

# Scarab is `seed_agent` — stop creating a second crate

> Operator's local checkout had a `crates/scarab/` with parallel
> `claim.rs`, `trainer.rs`, `config.rs` files duplicating
> `crates/trios-igla-race/`. That fork drifted: it kept the
> `AND account = $1` filter while `trios-igla-race/pull_queue.rs`
> shipped the **fungible** claim (no account filter). When `acc3`
> died at 2026-04-30 18:15 UTC, `seed=45` starved waiting for it
> while `acc5` idled. This document is the brake on creating a
> second worker crate ever again.

## Status

`crates/trios-igla-race/` already contains the full Stateless Scarab
Pattern stack. **No second crate is needed.**

| Concern | Where it lives | Fungible? |
|---|---|---|
| Claim SQL | `pull_queue.rs::pull_experiment()` | ✅ no `WHERE account = $N` |
| Heartbeat | `pull_queue.rs::spawn_heartbeat()` | ✅ uses `worker_id` |
| Trainer dispatch | `bin/seed_agent.rs::worker_tick()` | ✅ subprocess, env-driven |
| ASHA / EMA / sampler | `asha.rs`, `ema.rs`, `sampler.rs` | shared |
| `[[bin]]` entries | 6 already in Cargo.toml | adding scarab is one row |

The binary is called `seed_agent` for historical reasons. To deploy
it as `scarab-pool` on Railway, simply rename the service — the binary
itself does not care.

## How to deploy

```bash
# 1. Build the existing binary (no rename, no second crate needed)
cargo build --release --bin seed_agent -p trios-igla-race

# 2. Push image (one Dockerfile, all six accounts share it)
docker build -f bin/seed-agent/Dockerfile.scarab -t \
  ghcr.io/ghashtag/trios-seed-agent-real:latest .
docker push ghcr.io/ghashtag/trios-seed-agent-real:latest

# 3. Railway deploy — same image, six replicas (or twelve, etc.)
for ACC in 0 1 2 3 4 5; do
  railway --account=acc$ACC up --service scarab-pool
done
```

The only env var that matters is `NEON_DATABASE_URL`. `RAILWAY_ACC`
remains as a cosmetic log tag (so you can grep heartbeats by account)
but **does not steer claim**.

## What NOT to do

❌ Create `crates/scarab/` with copy-pasted `claim.rs`, `trainer.rs`,
   `config.rs`. The duplication will drift within one PR cycle.
   Confirmed bug pattern: 2026-04-30 18:15 UTC, when the local
   `scarab/src/claim.rs` carried the legacy `AND account = $1` while
   `trios-igla-race/src/pull_queue.rs` had already removed it.

❌ Add a thin `bin/scarab.rs` that calls into `seed_agent::worker_tick`.
   `worker_tick` is currently a `bin/`-private function; making it
   public would expose its `Cli` struct and tracing init. If you must
   have a second binary name, **rename the existing `seed_agent` bin
   to `scarab` in Cargo.toml** instead of cloning it.

❌ Maintain a second `Dockerfile.scarab` whose `COPY --from=…` paths
   diverge from `Dockerfile.real-seed-agent`. One Dockerfile per
   stateless worker is enough.

## What TO do (if rename is desired)

If the operator strongly wants `scarab` as the binary name (e.g. for
Railway service naming hygiene), the **only** correct edit is:

```toml
# crates/trios-igla-race/Cargo.toml
[[bin]]
name = "scarab"          # was: "seed_agent"
path = "src/bin/seed_agent.rs"
```

That's a one-line rename. Logic unchanged. CI still green. Image
still single. Tracking issue: trios-railway#101 Khepri umbrella.

## Audit trail

- `crates/trios-igla-race/src/pull_queue.rs:111-114` — fungible claim
  (`WHERE status='pending'` only)
- `crates/trios-igla-race/src/bin/seed_agent.rs:84-93` — calls
  `pull_experiment(worker_id)` without account
- `bin/seed-agent/src/claim.rs::CLAIM_SQL` (in the **other** crate
  `bin/seed-agent`) — also fungible after PR #106
- 2026-04-30 19:33 UTC merges:
  - trios-railway#106 — fungible `bin/seed-agent`
  - trios-trainer-igla#64 — fungible `scarab.rs` in trainer repo

## Anchor

`phi² + phi⁻² = 3` · TRINITY · NEVER STOP.
One crate. One claim path. One image. One worker.

# F2 Adapter — Scarab Integration Design

**Loop 16 QQ** | Offline-only design (no DB access required) | Status: ready for review

## Goal

Run F2 protocol (phi-ladder vs format-zoo BPB comparison, Issue #1021) on Railway scarab
infrastructure **without breaking the production scarab_strategy contract**.

## Constraint Analysis

The production scarab schema (`migrations/0007_scarab_strategy.sql`) is intentionally narrow:

```sql
optimizer  TEXT CHECK (optimizer IN ('adamw', 'muon', 'muon-cwd'))
format     TEXT CHECK (format IN ('fp32', 'bf16', 'gf16'))
status     TEXT CHECK (status IN ('active', 'paused', 'disabled'))
```

F2 needs to vary:
- **arm** ∈ {phi, zoo, fp32}
- **precision_bits** ∈ {1.58, 2.0, 3.0, 4.0, 8.0, 32.0}
- **quantizer** ∈ {paretoq_seq, paretoq_lsq, int4_rtn, bf16_e4m3, fp32_baseline}
- **task_kind** ∈ {counter, sparse_parity, bytes_file}
- **use_ffn**, **d_hidden**, **iso_neff_target_n**

None of these fit the production schema.

## Design: companion table + canonical name encoding

### Option A — Companion table (chosen)

Create `f2_strategy` table that references `scarab_strategy.service_id` (loop 16 migration
`0008_f2_strategy.sql`). Production scarabs continue using existing format='fp32'/'bf16'/'gf16'.
F2 scarabs **set scarab_strategy.format='fp32'** as a no-op placeholder and read their real
config from `f2_strategy` row.

**Pros:**
- Zero changes to production scarab_strategy schema
- F2-specific changes isolated to one migration
- View `f2_experiment_status` joins both tables for monitoring

**Cons:**
- F2 scarab binary needs different polling logic (two queries instead of one)
- Production scarab will see 'fp32' format and possibly tries to run that config — F2 scarab
  must IGNORE scarab_strategy.format and rely on f2_strategy instead

### Option B — Canon name encoding (rejected)

Encode F2 params in `scarab_strategy.canon_name` as a structured string:
`"F2-PHI-P158-N8K-TASKsp-FFN1-ISO0"`. F2 scarab parses the canon at startup.

**Pros:**
- No new table; uses existing fields
- Production scarab_strategy contract untouched

**Cons:**
- canon_name parsing is brittle — any typo breaks the experiment
- No DB-side schema enforcement (CHECK constraints)
- Hard to query "all phi at P=1.58" without LIKE patterns
- Adding a new param requires updating every existing canon string

**Decision**: Option A. The migration cost is low; the auditability win is large.

## F2 Scarab Binary (sketch)

```rust
// crates/trios-igla-race/src/bin/f2_scarab.rs
// Polls BOTH scarab_strategy + f2_strategy for the current service_id.

const F2_STRATEGY_SQL: &str = r#"
    SELECT s.seed, s.steps, f.arm, f.precision_bits, f.quantizer, f.task_kind,
           f.task_params, f.use_ffn, f.d_hidden, f.iso_neff_target_n,
           f.config_fingerprint
    FROM public.scarab_strategy s
    JOIN public.f2_strategy f USING (service_id)
    WHERE s.service_id = $1 AND s.status = 'active'
"#;

// On hash change → restart training with F2-specific args:
//   --arm phi --precision 1.58 --quantizer paretoq_seq
//   --task sparse_parity --task-params '{"n_bits":40,"k":3,"n_tasks":64}'
//   --use-ffn --d-hidden 64
//   --iso-neff-target-n 41152  (or omit for iso-N)
```

## BPB Sample Sink

BPB samples already flow to `ssot.bpb_samples` (per loop 14 user description).
F2 adapter needs the sink table to include enough metadata to identify the F2 experiment:

**Recommended addition** (separate migration, not in 0008):
- `bpb_samples.f2_service_id TEXT` — FK to f2_strategy.service_id
- Index on `f2_service_id, step`

Then Queen Hive can compute aggregated BPB per F2 experiment via simple JOIN.

## Queen Hive Integration

Queen Hive writes to `f2_strategy` (via `upsert_f2_strategy` helper function) to launch
new F2 experiments. Each row creates one (arm, precision) point in the F2 sweep matrix.

To launch a full sweep at iso-N (loop 15 correct comparison):
```sql
-- 7 scarabs × 6 points = 42 row inserts. With 7 Railway accounts, ~6 sweeps in parallel.
SELECT upsert_f2_strategy('scarab-1', 'fp32', 32.0, 'fp32_baseline', 'sparse_parity', ...);
SELECT upsert_f2_strategy('scarab-2', 'phi', 1.58, 'paretoq_seq', 'sparse_parity', ...);
-- ... etc
```

Queen Hive then polls `f2_experiment_status` view to monitor progress and harvest BPB
samples from `bpb_samples`.

## Verification Plan

1. **Migration apply** (when DB credentials work):
   `psql $DATABASE_URL -f migrations/0008_f2_strategy.sql`
2. **Insert one test row** to verify CHECK constraints + helper function.
3. **Build f2_scarab binary** + push to one Railway service.
4. **Watch f2_experiment_status view** — heartbeat should appear within poll_interval.
5. **Check ssot.bpb_samples** for samples tagged with the new service_id.

## Open Questions

1. Does production `ssot.bpb_samples` table have `service_id` column already? If not,
   additional migration needed.
2. Do existing scarabs poll `public.scarab_strategy` ONLY, or also have hardcoded
   `format` fallbacks?
3. Railway account quota: can we run 7 services simultaneously per account, or 1?

## Compute-blocker resolution

When this design ships + DB credentials work, Issue #1021 compute-blocker dissolves:
- 7 scarabs × 6 F2 points = 42 parallel champion-scale runs
- Each scarab honest sparse-parity + 2-layer FFN per loop 16 OO+PP
- BPB samples flow to ssot, Queen Hive aggregates
- F2 verdict (Welch / TOST / Pareto / Bayes / Permutation) computed offline from samples

# LANE Registry — R7 Falsifiability Spec

**Status**: pre-registered fleet sweep specification
**Closes**: [gHashTag/trios#774](https://github.com/gHashTag/trios/issues/774)
**Anchor**: φ² + φ⁻² = 3 · R5-HONEST · R7-FALSIFIABLE
**Defense deadline**: 2026-06-15

---

## TL;DR

Before any row in `ssot.bpb_samples` is allowed to be labelled `champion_*`, the fleet **MUST** have covered:

| Axis | Minimum unique values | Pre-registered set |
|---|---|---|
| **LR** (learning rate) | **≥ 3** | `{1e-5, 1e-4, 1e-3}` |
| **rng** (random seed) | **≥ 3** | `{1597, 2584, 4181}` (Fibonacci, anti-collision) |
| **LANE** | **≥ 1** | `{SHORT-WAVE-MATRIX, LONG-WAVE-MATRIX, …}` |
| **rows in 6h** | ≥ 50 | (writer-health smoke) |

Failure → gardener `Stage 0e (Diversity Gate)` is **RED** and champion declaration is **suppressed**.

## Canonical name pattern (`leaderboard-snapshot` v1.0)

```regex
^IGLA-(?P<lane>[A-Z0-9-]+)-(?P<fmt>[a-z0-9]+)-h(?P<h>\d+)-LR(?P<lr>[0-9.eE+\-]+)-rng(?P<rng>\d+)-(?P<algo>[a-z0-9]+)$
```

Example: `IGLA-SHORT-WAVE-MATRIX-gf256-h128-LR0.0001-rng1597-adamw`

## Falsification criterion (Popper-style, R7)

Champion declaration `H₁ = "config C beats all others"` is **falsifiable** iff:

1. **Seed ablation**: same config run on `seed ∈ {s₁, s₂, s₃}` — paired t-test on BPB gives `p < 0.05` against the runner-up.
2. **LR perturbation**: same config run on `LR ∈ {LR_champ × 0.1, LR_champ, LR_champ × 10}` — champion still wins or remains within `+0.02 BPB` of best-LR config (no LR cliff).
3. **Steps**: champion ran ≥ `min_steps` (currently 76k for SHORT-WAVE-MATRIX).
4. **Diversity gate green**: `trios-lane-diversity-gate --window-hours 6` exits 0.

If any of (1)-(4) fails → champion is `un-honest` and skipped on `/leaderboard`.

## Pre-registered acceptance (Gate G2, R7)

```bash
trios-lane-diversity-gate \
  --window-hours 6 \
  --min-lr 3 --min-rng 3 --min-lane 1 --min-rows 50

# Expected stdout (GREEN):
# {"verdict":"green","uniq_lr":3,"uniq_rng":3,...}

# Exit 0 → gardener allowed to compute champion
# Exit 1 → gardener MUST skip champion_* writes (R7 RED)
# Exit 2 → infra error → fail-closed (treat as RED)
```

## Integration with `tri-gardener` (Stage 0e)

Add to `bin/tri-gardener/src/main.rs` (or wherever `tri-gardener` 2.9 lives) before any `champion_declared` log line:

```rust
// Stage 0e: R7 diversity gate (Closes #774)
let status = std::process::Command::new("trios-lane-diversity-gate")
    .arg("--neon-url").arg(&railway_pg_url)
    .arg("--window-hours").arg("6")
    .status()
    .await?;

let gate_green = status.success();
if !gate_green {
    warn!("Stage 0e: diversity gate RED — skipping champion declaration");
    record_audit_run("diversity_gate", "red", evidence_json).await?;
    return Ok(());
}
record_audit_run("diversity_gate", "green", evidence_json).await?;
```

## Fleet sweep minimal config

```
formats:    {bf16, fp16, gf16, gf256, int4}                       # 5
algos:      {adamw, muon}                                         # 2
LRs:        {1e-5, 1e-4, 1e-3}                                    # 3
seeds:      {1597, 2584, 4181}                                    # 3
lanes:      {SHORT-WAVE-MATRIX}                                   # 1 (baseline)
─────────────────────────────────────────────────────────────────
Total trainers required: 5 × 2 × 3 × 3 × 1 = 90 (vs current 12)
```

## R5 evidence

Every gate invocation emits a JSON line to stdout that the gardener writes to `audit_runs(probe='diversity_gate', verdict, evidence)` per migration `0003_zenodo_sentinel_audit_runs.sql` (merged 2026-05-13 in PR #142).

## Sibling skills

- `leaderboard-snapshot` v1.0 — canon-name regex + Matrix-mode display
- `igla-honest-short-run` v1.0 — per-trainer pre-flight (O(1))
- `tri-gardener-runbook` v2.9 — fleet orchestrator (calls Stage 0e)
- `zenodo-sentinel` v1.1 — hourly cross-cutting probes

## References

- Popper, K. *The Logic of Scientific Discovery* (1959), §4 on falsifiability
- IGLA-RACE design doc, Appendix B (Popper falsification appendix in monograph)
- φ² + φ⁻² = 3 (algebraic identity anchor)

🤖 Authored by Trinity Queen Hive · 2026-05-13

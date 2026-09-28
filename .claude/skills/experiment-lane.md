# Experiment Lane Skill

Skill for creating, running, and verifying experiment lanes in the IGLA RACE system.

## Overview

Every experiment follows the **Lane structure** with falsifier, acceptance gates, and anti-fakery rules.

**Reference:** gHashTag/trios#645 — GPTQ on GF16 one-shot (exemplar lane)  
**External SOTA:** openai/parameter-golf#2135 — 1.05651 BPB (replication target)

## Lane Structure Template

```
Lane L-NN-X:
  - Deliverable: <what is produced>
  - Acceptance: <measurable criteria>
  - Falsifier H0: <null hypothesis>
  - If H0 cannot be rejected: <publishable negative result>
```

## Example: L-26 (GPTQ on GF16)

| Lane | Deliverable | Acceptance |
|------|-------------|------------|
| **L-26-A** Coq invariant | `coq/Trios_GPTQ_GF16.v` proving `gptq_correct ∘ Q_GF16` preserves invariant | `coqc` clean, witness JSON |
| **L-26-B** Rust impl | `gf16_quantize_matrix_gptq(W, X_batches, calibration_n)` | `cargo test` green; MSE ≤ baseline on 3 random PSD |
| **L-26-C** 3-seed ablation | bin `gptq_calibration_ablation` runs 3×3 grid | 9 rows + paired-t analysis in `assertions/calibration_ablation.jsonl` |

## Algorithm Specification Template

Every lane MUST include the exact algorithm:

```
input:  W ∈ R^{rows × cols}
        X ∈ R^{cols × n_samples}
        Q : R^{rows} → FORMAT   (black-box quantiser)
        λ : f32                 (dampening)

H ← 2 · X · X^T  + λ·I
L ← Cholesky(H)
H_inv ← solve_triangular(L, I)·solve_triangular(L^T, I)

for j = 0..cols-1:
    w_j ← W[:, j]
    q_j ← Q(w_j)
    err ← (w_j − dequant(q_j)) / H_inv[j, j]
    W[:, j+1..] -= err · H_inv[j, j+1..]
    Q_OUT[:, j] ← q_j

return Q_OUT
```

## Acceptance Gates (Mandatory)

| Gate | Check |
|------|-------|
| **G1** | `cargo check --all-targets` clean |
| **G2** | Unit tests green (reconstruction-MSE invariant or equivalent) |
| **G3** | `coqc` clean, no `Admitted.` (if Coq lane exists) |
| **G4** | Binary produces required output files |
| **G5** | Statistical analysis printed to stdout (t-stat, p, verdict) |
| **G6** | All CI checks green |

## Anti-Fakery Rules (Mandatory)

- Calibration data MUST come from **training shards only** (no validation peek)
- All ablation cells must be **independently replayable**
- Baseline (`N=0`) must be **byte-identical** to current implementation (sanity assert)
- Paired-t rows must include **raw per-seed Δ values** (no summary-only)
- No claim of "lift confirmed" unless `p<0.25` paired-t reaches BOTH comparisons
- Failing the falsifier IS a valid result — document it honestly per R5

## R-Discipline

| Rule | Meaning |
|------|---------|
| R1 | Rust-only (no Python in `src/`) |
| R3 | PR-only (no direct push to main) |
| R4 | Trace (every cell timestamped + sha-pinned) |
| R5 | Honest (falsifier explicitly stated) |
| R7 | Witness (`assertions/*.jsonl`) |
| R8 | Falsifier (null hypothesis stated upfront) |
| R10 | Atomic (single-purpose PR per lane) |
| R12 | Reversible (baseline path preserved as default) |

## Files to Create / Modify

Standard file set for every lane:

```
coq/Trios_<NAME>.v                              NEW (~250 lines)
crates/trios-<crate>/src/<feature>.rs            NEW (~180 lines)
crates/trios-<crate>/src/lib.rs                  +pub use <feature>::*
crates/trios-<crate>/tests/<feature>_*.rs        NEW (~120 lines)
src/bin/<name>_ablation.rs                       NEW (~220 lines)
assertions/<name>_ablation.jsonl                 NEW (9+ rows)
assertions/coq_runtime_invariants.json           APPEND 1 entry
docs/wave<NN>_<name>.md                          NEW report
MIGRATION.md / CHANGELOG.md                      +1 line
```

## PR Mechanics

- **Title:** `feat(<area>): <what> (replicates parameter-golf#<N> lever on <platform>)`
- **Branch:** `feat/<name>` off `main`
- **Body:** Why · External reference · Falsifier · Lane summary · Acceptance gates · Anchor
- **Labels:** `enhancement`, `P1`, `experiment`
- **Merge:** Squash-merge, delete branch, **no `--admin`**

## Parameter-Golf Integration

When replicating parameter-golf findings:

1. Read the PR diff to extract the exact lever
2. Identify if the lever is **algorithm-agnostic** (works with any quantiser) or **format-specific**
3. Port the inner loop to Rust, plugging our format's `quantize_matrix` as `Q`
4. Run 3-seed ablation with paired-t statistical test
5. Document both success AND failure honestly

### Current SOTA to Replicate

| PR | BPB | Lever | Status |
|----|-----|-------|--------|
| #2135 | 1.05651 | GPTQ_CALIBRATION_BATCHES 16→32 | Replicating in #645 |
| #2146 | audit | grace-policy audit | Merged upstream |
| #2139 | — | TTT Peer-LoRA Ensemble | Novel technique, not measured |

## Quick Commands

```bash
# Create new lane from template
# (copy L-26 structure and modify)

# Run ablation binary
cargo run --release --bin gptq_calibration_ablation

# Verify paired-t on stdin replay
cat assertions/calibration_ablation.jsonl | python3 scripts/paired_t.py

# Check Coq clean
coqc coq/Trios_GPTQ_GF16.v

# Check assertions schema
python3 scripts/verify_assertions.py assertions/calibration_ablation.jsonl
```

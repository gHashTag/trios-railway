# L-26: GPTQ on GF16 — Experiment Specification

**Source:** trios#645 + parameter-golf#2135  
**Goal:** Replicate the GPTQ_CALIBRATION_BATCHES lever on GF16 quantization  
**Hypothesis:** Hessian-corrected GF16 quantization improves BPB over naive single-pass scale-fit  
**Falsifier H0:** GPTQ-correction with N∈{16,32} gives no significant BPB improvement over naive GF16 (paired-t one-tail p≥0.25 across seeds {47,89,144})

---

## Algorithm

```
input:  W ∈ R^{rows × cols}
        X ∈ R^{cols × n_samples}  (N calibration batches from training shards)
        Q : R^{rows} → GF16       (gf16_quantize_matrix as black-box)
        λ : f32                   (dampening, default 1e-2 · trace(H)/cols)

H ← 2 · X · X^T  + λ·I
L ← Cholesky(H)
H_inv ← solve_triangular(L, I) · solve_triangular(L^T, I)

for j = 0..cols-1:
    w_j ← W[:, j]
    q_j ← Q(w_j)
    err ← (w_j − dequant(q_j)) / H_inv[j, j]
    W[:, j+1..] -= err · H_inv[j, j+1..]
    Q_OUT[:, j] ← q_j

return Q_OUT
```

**Critical:** With `calibration_n = 0`, function MUST be byte-equivalent to current `gf16_quantize_matrix`.

---

## Lane Structure

| Lane | Deliverable | Acceptance |
|------|-------------|------------|
| **L-26-A** | Coq invariant `Trios_GPTQ_GF16.v` | `coqc` clean, no `Admitted.` |
| **L-26-B** | Rust impl `gf16_quantize_matrix_gptq` | `cargo test` green; MSE ≤ baseline on 3 random PSD |
| **L-26-C** | Ablation binary + paired-t analysis | 9 rows (3 seeds × 3 N values) + verdict in `assertions/calibration_ablation.jsonl` |

---

## Files to Create / Modify

```
coq/Trios_GPTQ_GF16.v                              NEW (~250 lines)
crates/trios-golden-float/src/gptq.rs              NEW (~180 lines)
crates/trios-golden-float/src/lib.rs               +pub use gptq::*
crates/trios-golden-float/tests/gptq_reconstruction.rs   NEW (~120 lines)
src/bin/gptq_calibration_ablation.rs               NEW (~220 lines)
assertions/calibration_ablation.jsonl              NEW (10 rows: 9 grid + 1 verdict)
assertions/coq_runtime_invariants.json             APPEND 1 entry
docs/wave26_gptq_on_gf16.md                        NEW report
```

---

## Acceptance Gates

| Gate | Check |
|------|-------|
| **G1** | `cargo check --all-targets` clean |
| **G2** | Unit tests green (reconstruction-MSE invariant) |
| **G3** | `coqc` clean, no `Admitted.` |
| **G4** | Binary produces `assertions/calibration_ablation.jsonl` |
| **G5** | Paired-t analysis printed: t-stat, p, verdict at p<0.25 for (N=0 vs 16) and (N=16 vs 32) |
| **G6** | All CI checks green |

---

## Anti-Fakery Rules

- Calibration data from **training shards ONLY** (no validation peek)
- All 9 cells independently replayable
- `N=0` baseline byte-identical to current `gf16_quantize_matrix`
- Raw per-seed Δ values in paired-t output
- No "lift confirmed" claim unless p<0.25 for BOTH comparisons
- Failing falsifier IS valid — document honestly per R5

---

## R-Discipline

R1 Rust-only · R3 PR-only · R4 trace · R5 honest · R7 witness · R8 falsifier · R10 atomic · R12 reversible

---

## Coq Invariant (L-26-A Sketch)

```coq
Theorem gptq_correct_preserves_error :
  forall (W : matrix) (X : matrix) (Q : vector -> gf16),
  let H := 2 * X * X^T + lambda * I in
  let L := cholesky H in
  let H_inv := solve_triangular L I * solve_triangular L^T I in
  let W_gptq := gptq_loop W H_inv Q in
  let W_naive := map Q W in
  norm(W * X - dequant(W_gptq) * X) <= norm(W * X - dequant(W_naive) * X).
```

---

## PR Mechanics

- **Branch:** `feat/gptq-on-gf16` off `main`
- **Title:** `feat(gf16): port GPTQ Hessian-correction with GF16 quantiser (replicates parameter-golf#2135 lever on CPU)`
- **Labels:** `enhancement`, `P1`, `experiment`
- **Merge:** Squash-merge, no `--admin`

---

## Battle Cry

`phi² + phi⁻² = 3 · TRINITY · PORT THE LEVER · PROVE OR FALSIFY ON GF16`

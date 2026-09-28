# IGLA RACE — Master Skill

Complete skill for operating the IGLA RACE distributed training hunt.

## Anchor

`phi² + phi⁻² = 3 · TRINITY · O(1) FOREVER`

## Overview

**Repository:** `gHashTag/trios` + `gHashTag/trios-trainer-igla` + `gHashTag/trios-railway`  
**Issue #143:** IGLA RACE v2 — ONE SHOT DASHBOARD (NEVER CLOSE)  
**Issue #446:** EPIC — WAVE-GF-001 GoldenFloat × strategy sweep  
**Target:** BPB < 1.50 on 3 seeds (43, 44, 45) with p < 0.01  
**Current best:** BPB = 2.18 (h=828 attn=2L seed=43 81K steps)

## Coq Invariants (INV-1..INV-10)

| ID | Theorem | Status | Effect |
|----|---------|--------|--------|
| INV-1 | `bpb_decreases_with_real_gradient` | partial | fixes TASK-5D |
| INV-2 | `asha_champion_survives` | **PROVEN** | threshold=3.5=phi²+phi⁻²+0.5 |
| INV-3 | `gf16_safe_domain` | Lucas proven | -40% configs |
| INV-4 | `nca_entropy_stability` | **PROVEN** | band_width=1 |
| INV-5 | `lucas_closure_gf16` | n=1,2 proven | GF16 consistency |
| INV-6 | `ema_decay_valid` | TODO | -20% configs |
| INV-7 | `igla_found_criterion` | TODO | victory iff 3-seed BPB<1.50 |
| INV-8 | `lr_phi_band` | **PROVEN** | lr=0.004=alpha_phi/phi³ |
| INV-9 | `qk_gain_phi_sq` | **IMPLEMENTED** | qk_gain=PHI_SQ=2.618 |
| INV-10 | `asha_rungs_trinity` | TODO | rungs=1000·3^k |

### phi-anchored Parameters

| Parameter | Value | Derivation |
|-----------|-------|------------|
| `bpb_prune_threshold` | 3.5 | phi² + phi⁻² + 0.5 |
| `NCA grid` | 81 | 3⁴ |
| `NCA K states` | 9 | 3² |
| `lr champion` | 0.004 | alpha_phi / phi³ |
| `d_model` | 384 | ~3⁴ × phi³ |
| `qk_gain` | 2.618 | PHI_SQ |
| `alpha_phi` | ~0.118 | phi⁻³/2 |

## BPB Matrix (Format × Algorithm)

**Coverage:** ~50/312 cells (16%). Target: 100% for PhD thesis.

Key findings:
- **16-bit triple tie:** gf16 = bf16 = fp16 = 2.566 BPB
- **gf16 wins on φ-distance:** 0.049 vs bf16 0.525 (10.7× better alignment)
- **AdamW wins** low-bit (4-12 bit): gf4, gf8, gf12, int8, fp8
- **Muon wins** 16-32 bit: fp32, gf32, fp16
- **Best measured:** gf256 @ 2.5586 BPB (single result)

### Algorithm Pattern Hypothesis

Muon orthogonalization needs ≥16 mantissa bits for gradient curvature.

## Victory Conditions (Issue #143 Closure)

Issue #143 closes ONLY when ALL true:

1. BPB < 1.50 on seeds 43, 44, 45 (three independent runs)
2. p < 0.01 statistical significance
3. `cargo test --workspace` = GREEN
4. Neon: `status='winner'`
5. `coqc trinity-clara/proofs/igla/*.v` = GREEN (INV-001..010)
6. Commit: `git commit -m "IGLA FOUND: BPB=X.XXXX seed=43,44,45"`
7. `git push origin main`

## Constitutional Laws (L-R1..L-R14)

| Law | Rule | Violation → |
|-----|------|-------------|
| L-R1 | RUST ONLY — no .py, .sh, .ipynb | REVERT |
| L-R2 | WORKERS=4-16 via env var | REVERT |
| L-R3 | Every result → Neon + `.trinity/experience/` | LESSON MISSING |
| L-R4 | `cargo test --workspace` = GREEN before push | PR BLOCKED |
| L-R5 | `cargo clippy -- -D warnings` = 0 | PR BLOCKED |
| L-R6 | SIGTERM → graceful shutdown | DATA LOSS |
| L-R7 | Neon query timeout <= 30 sec | WORKER CRASH |
| L-R8 | Trainer stdout: ONLY `BPB=X.XXXX` | PARSE FAIL |
| L-R9 | GF16 only with d_model >= 256 | +3.21 BPB |
| L-R10 | T-JEPA ASHA min rung = 3000 steps | FALSE PRUNE |
| L-R11 | NCA entropy [1.5, 2.8] = hard loss penalty | COLLAPSE |
| L-R12 | All agents -> branch `main` ONLY | CONFLICT |
| L-R13 | `agent_id` + `branch='main'` in every Neon record | DASHBOARD FAIL |
| **L-R14** | **`coqc` = exit 0 before race** | **RACE INVALID** |

## Key Findings (Agent ALPHA Experience)

1. **T1-02 (2-layer HybridAttn + ReLU²)** — only lever that worked: -0.35 BPB
2. **Muon FALSIFIED** — NS-1 gives +0.11 vs AdamW. NS-5 too slow on CPU
3. **JEPA doesn't help** — predictor gradients don't flow to encoder
4. **NCA doesn't help** — loss scalar has no gradient connection to weights
5. **Attention has no backward** — `model_hybrid_attn.rs` only implements `forward()`
6. **Architecture ceiling** — 338K params plateaus at ~2.15. Need h=2000+ or more layers

## BPB Roadmap (Actual Results)

| Step | Technique | Expected Δ | Actual Δ | Target |
|------|-----------|------------|----------|--------|
| Baseline | 6-gram h=384 | — | — | 2.5329 |
| T1-02 | Attention + ReLU² (2L) | -0.30 | **-0.35** | 2.18 ✅ |
| T2-01 | Muon optimizer | -0.15 | **+0.11** | ❌ FALSIFIED |
| T2-02 | NCA auxiliary | -0.15 | **~0** | ❌ no grad flow |
| T2-04 | QK-Gain phi² | -0.10 | in model | ✅ implemented |
| T2-07 | ReLU² activation | -0.08 | in model | ✅ implemented |
| **Missing** | **Attention backward** | **?** | **?** | **KEY LEVER** |
| **Missing** | **Scale up (h=2000+)** | **?** | **?** | **KEY LEVER** |

## External References

- **Parameter Golf SOTA:** openai/parameter-golf#2135 — 1.05651 BPB (GPTQ_CALIBRATION_BATCHES=32)
- **Algorithm paper:** arXiv 2512.23675 — End-to-End Test-Time Training
- **Coq theorems:** trinity-clara/proofs/igla/
- **SoT trainer:** gHashTag/trios-trainer-igla/SOURCE_OF_TRUTH.md
- **Anchor DOI:** Zenodo 10.5281/zenodo.19227877

## Quick Commands

```bash
# Check Coq invariants
coqc trinity-clara/proofs/igla/*.v

# Run full test suite
cargo test --workspace

# Clippy zero warnings
cargo clippy --all-targets --all-features -- -D warnings

# Check IGLA race status
python3 scripts/fleet_guardian.py --env .env

# View dashboard
cat FLEET_DASHBOARD.md
```

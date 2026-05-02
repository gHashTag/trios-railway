---
name: igla_canon_spec
description: IGLA canon naming spec violation (session 2026-05-02)
type: feedback
---

## Rule: IGLA Canon Naming Spec (Session a917e07b, INV-12)

### Format
```
IGLA-<TYPE>-<NUM>-<TAG>-seed<N>
```

| Field | Values |
|-------|--------|
| IGLA | namespace (fixed) |
| TYPE | HYBRID, JEPA-T, NCA, PHI, TRAIN_V2, TRINITY3K, TJEPA, MUON |
| NUM | FP32, GF16, BF16, DLFLOAT, FP8E4M3, FP8E5M2, GF8, GF32, GF64, GFTERN, FP16 |
| TAG | CHAMP, WSD, BS8, GRADFIX, EMA10, h512, h768, E2E (probe), ... |
| seed<N> | RNG + unique slot, reserved by TAG range |

### Seed Ranges (Tripwire #98)
| Range | Tag | Status |
|-------|-----|--------|
| 42..45 | CHAMP | 🔒 locked |
| 200..202 | WSD | active |
| 210..212 | BS8 | active |
| 220..222 | JEPA-T GRADFIX | active |
| 230..232 | NCA GRADFIX | active |
| 240..242 | EMA10 | active |
| 250..252 | h512 | active |
| 260..262 | h768 | active |
| 300..302 | GF16 TRAIN_V2 | Phase 4 |
| 310..312 | GF16 HYBRID | Phase 4 |
| 320..322 | GF16 PHI | Phase 4 |
| 330..331 | DLFloat/BF16 | Phase 5 |
| 340..341 | FP8 | Phase 5 |
| 1597 | PROBE/E2E | 🔴 violation - not in map |

### Violations (2026-05-02 session)
| Probe | Used canon | Should be |
|-------|------------|-----------|
| 2030 | PROBE-VERIFY-BPBSAMPLE-9efdd9a3 | IGLA-HYBRID-FP32-PROBE-seed1597 |
| 2031 | PROBE2-FINAL-METRICS-9efdd9a3 | IGLA-HYBRID-FP32-PROBE-seed1597 |
| 2033 | probe-2031-tiny-shakespeare-e2e | IGLA-HYBRID-FP32-E2E-seed1597 |

### What to do
For probe 2034, use: **IGLA-HYBRID-FP32-E2E-seed1597** (or reserve PROBE tag range)

**Why:** Manual canon names in INSERT statements must follow spec. CI Tripwire #98 blocks out-of-range seeds.

**How to apply:** Always use `IGLA-<TYPE>-<NUM>-<TAG>-seed<N>` format when creating strategy_queue entries.

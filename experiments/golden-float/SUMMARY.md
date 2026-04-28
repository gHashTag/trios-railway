# Golden Float Family Experiments - Summary

Generated: 2026-04-28
Status: 5 experiment TOMLs prepared

## Checklist

| # | Format | Config | Status | BPB | Verdict |
|---|---|---|---:|---:|
| G1 | GF8 (8-bit) | ⏳ Not started | — | — |
| G2 | GF16 (16-bit) | ⏳ Not started | — | — |
| G3 | GF32 (32-bit) | ⏳ Not started | — | — |
| G4 | GF64 (64-bit) | ⏳ Not started | — | — |
| G5 | GFTernary (2-bit) | ⏳ Not started | — | — |

## Progress Against Whitepaper Benchmarks

| Benchmark | Whitepaper Result | Our Status |
|----------|------------------|-------------|
| BENCH-001 | GF16 ≈ fp16, 2× bf16 | ✅ GF16 baseline in `experiments/smoke-test/E5-gf16-storage.toml` |
| BENCH-002 | GF16 add: 7.2 ns/op | ⏳ Pending implementation |
| BENCH-003 | GF16 5.80% synthetic | ⏳ Pending frozen weights test |
| BENCH-004a | GF16 11.86% random | ⏳ Pending initialized weights test |
| BENCH-004b | GF16 97.67% MNIST = f32 | ⏳ Pending full 120K validation |
| BENCH-005 | GF16 118 LUT + 94 LUT + 1 DSP | ⏳ Pending FPGA synthesis |
| BENCH-006 | GF16 71 LUT + 16 DSP (16-dot) | ⏳ Pending FPGA synthesis |

## Next Steps

1. Implement GF16 in trios-trainer-igla (BENCH-001..004)
2. Run G2 (GF16-baseline) for full 120K validation
3. If G2 passes: Enable TRAIN-001 (GF16 pipeline)
4. Implement GF32/GF64 specs in zig-golden-float
5. Validate hybrid feasibility with G5

## Related Work

- Smoke-first experiments: `../smoke-test/` — E4-E7, 10-min orthogonal signal
- Seed policy DB lock: `migrations/2026-04-28-seed-policy.sql` — E.2 complete

---

`φ² + φ⁻² = 3` · GOLDEN FLOAT FAMILY READY

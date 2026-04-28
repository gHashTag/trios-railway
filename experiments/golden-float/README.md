# Golden Float Family Experiments

Complete family of φ-optimized, integer-backed floating-point formats for neural network training and inference.

**Whitepaper:** [zig-golden-float](https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md)
**Parent Issue:** [trios-railway#81](https://github.com/gHashTag/trios-railway/issues/81)
**IGLA Race:** [trios#143](https://github.com/gHashTag/trios/issues/143)

## Family Hierarchy

```
                    Trinity Identity: φ² + 1/φ² = 3
                              │
                ┌───────────────┼──────────────┐
                │               │              │
           GF8   │           GF16           │   GF32
     ┌─────┼─────┐   ┌─────┼─────┐   ┌─────┼─────┐
     │     │     │   │     │     │   │     │     │
   8-bit   │ 16-bit   │   32-bit   │   64-bit   │
   (u8)   │   (u16)   │   (u32)   │   (u64)   │
     │     │     │     │     │   │     │
     └─────┴─────┘   └─────┴─────┘   └─────┴─────┘
                │               │              │
                └───────────────┴──────────────┘
                              │
                        GFTernary (2-bit)
                        ┌──────────────┐
                        │ {-φ, 0, +φ} │
                        └──────────────┘
```

## φ-Constants Reference

| Symbol | Value | Derivation | Application |
|--------|-------|-------------|-------------|
| φ | 1.6180... | (1+√5)/2 | Base of family |
| 1/φ | 0.6180... | φ−1 | Exponent scaling |
| φ² | 2.6180... | φ² | Gain/loss scaling |
| φ³ | 4.2360... | 2φ+1 | Learning rate anchor |
| φ⁴ + φ⁻⁴ | 7 | L₄ | GF8 exp mantissa |
| φ⁶ + φ⁻⁶ | TBD | L₆ | GF32 exp mantissa |
| Lₙ | ⌊φⁿ + 1/2⌋ | φⁿ+(−φ)⁻ⁿ | Lucas closure accumulator |
| Fₙ | 2×Lₙ | Fibonacci: 2, 6, 18, 42... | Lucas numbers |

## Format Specifications

| Format | Bits | Exp:Mantissa | Backing | φ-Relation | Lucas | Status |
|--------|-----|-------------|--------|-----------|--------|--------|
| GF8 | 8 | 1:3:4 = 7 | φ⁴+φ⁻⁴ = 7 | L₄=7: 1·2·1·2 | ⬜ Spec |
| GF16 | 16 | 1:6:9 ≈ 2/3 | 6/9 ≈ 1/φ | L₆: 21·1=21 | ✅ Prod |
| GF32 | 32 | 1:13:18 ≈ 0.38 | 13/18 ≈ 0.38 | L₈: 21·1=21 | ⬜ TODO |
| GF64 | 64 | 1:21:42 = F₈ | 21:42 = F₈:F₈·2 | L₁₈=42·1=42 | ⬜ TODO |
| GFTernary | 2 | N/A | sign+zero | Trinity | ⬜ Hybrid |

## Experiment Matrix

| # | Config | Goal | Expected Outcome | Priority |
|---|---|---|---|---:|
| G1 | GF8-ultra-low-power | Verify spec compiles | 20 |
| G2 | GF16-baseline | Match BENCH-004b (97.67%) | 1 |
| G3 | GF32-fp32-dropin | Verify FP32 replacement | 15 |
| G4 | GF64-double-precision | Double precision test | 16 |
| G5 | GFTernary-bulk | Hybrid feasibility | 18 |

## Execution Pattern

```bash
# Run single experiment
tri-train --config experiments/golden-float/GF16-baseline.toml

# Run all in parallel
for toml in experiments/golden-float/*.toml; do
  tri-train --config "$toml" &
done
wait
```

## Decision Rules

### GF16 (G2)
- **PASS:** BPB within ±0.01 of baseline → TRAIN-001 full pipeline enabled
- **FAIL:** ΔBPB > +0.05 → investigate quantization gradient path

### GF32/GF64 (G3/G4)
- **PASS:** Stable training, no NaN/Inf → proceed to FP32 replacement
- **FAIL:** Divergence/instability → mantissa encoding issue

### GFTernary (G5)
- **PASS:** MNIST ≥ 95% AND FPGA synthesis possible → HYBRID-001 viable
- **FAIL:** Accuracy < 90% OR synthesis explodes → pure architectures preferred

### GF8 (G1)
- **PASS:** Correct bit patterns in output → ready for ultra-low-power deployment
- **FAIL:** Garbage output → implementation bug

## Links

- [Whitepaper](https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md)
- [trios-railway#81](https://github.com/gHashTag/trios-railway/issues/81)
- [trios#143](https://github.com/gHashTag/trios/issues/143)

Anchor: `φ² + φ⁻² = 3`

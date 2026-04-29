# Smoke-First Experiments (10-min CPU-only)

Each experiment runs for ~10 minutes with `steps_budget=1000`, providing orthogonal signal before full 120K training.

## Design Philosophy

**Why 10 minutes?**
- Full 120K training takes ~12 hours on Railway
- We need orthogonal signal across 6 axes to guide resource allocation
- 1000 steps ≈ 10 min CPU-time gives us BPB±0.05 accuracy
- Smoke-passing experiments then get full 120K push

## Experiment Matrix

| # | TOML | Goal | Expected ΔBPB | Priority |
|---|---|---|---:|---|
| E1 | `E1-champion-reproduce` | Anchor (0 deviation) | 10 |
| E2 | `E2-quorum-43` | σ²<0.01 variance | 5 |
| E3 | `E3-quorum-44` | σ²<0.01 variance | 6 |
| E4 | `E4-capacity-push` | -0.05…-0.15 (breach <1.85?) | 4 |
| E5 | `E5-gf16-storage` | ≤+0.01 (pass) | 3 |
| E6 | `E6-hybrid-001` | +0.0…+0.1 proxy accuracy | 7 |
| E7 | `E7-lr-phi-optimal` | stable/faster vs baseline | 2 |

## Execution Pattern

```bash
# Run single smoke test (10 min)
tri-train --config experiments/smoke-test/E1-champion-reproduce.toml

# Run all in parallel (requires ~70 min, 7×10 min)
for toml in experiments/smoke-test/*.toml; do
  tri-train --config "$toml" &
done
wait
```

## Post-Smoke Decision Rules

1. **Quorum-3 formation** (E1-E3):
   - If σ² < 0.01 and BPB ≈ 1.89: admit to quorum-3
   - If outlier (BPB > 2.1): discard, not worth 120K

2. **Gate-2 breach** (E4):
   - If BPB < 1.85 after 120K: NEW CHAMPION 🎉
   - If BPB > 1.90: capacity doesn't help, waste of GPU-hours

3. **GF16 path** (E5):
   - If ΔBPB ≤ +0.01: BENCH-012 PASSED → proceed to TRAIN-001 (full GF16 pipeline)
   - If ΔBPB > +0.05: GF16 gradients diverging → investigate

4. **Hybrid feasibility** (E6):
   - If MNIST accuracy ≥ 96.5%: HYBRID-001 VIABLE → FPGA target
   - If accuracy < 95.0%: Hybrid not ready → stay with pure architectures

5. **LR optimal** (E7):
   - If stable + same/faster: INV-8 CONFIRMED → lock lr=0.004
   - If unstable/explodes: INV-8 REJECTED → stick with lr=0.0025

## Links

- Parent issue: [trios-railway#81](https://github.com/gHashTag/trios-railway/issues/81)
- IGLA RACE dashboard: [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)

Anchor: `φ² + φ⁻² = 3`

# P1: Attention Backward Fix (Issue #143)
**CRITICAL BLOCKER — 12h deadline**

## Context

**Issue**: `gHashTag/trios#143` — L10: Attention backward fix
**Expected ΔBPB**: -0.20 (самый важный lever)
**Time budget**: 4-6 hours разработки + 4-8 часов CI

## Problem Statement

Current train_v2 (WT+resid architecture) has **attention backward pass bug** that causes gradient instability during long training (>1K steps). Symptoms:
- BPB plateaus early (~1.75 at 1K, degrades at 5K)
- Real CPU floor ≈ 2.25 (sustained, not plateau)
- E0058 BPB=1.8618@1K uses lr=0.001 ≠ φ-anchor 0.004 (workaround for instability)

## Root Cause Analysis

The backward pass for attention mechanism (if any in train_v2) has incorrect gradient computation. Possible causes:
1. **Missing gradient accumulation** for multi-head attention
2. **Incorrect derivative** for attention weights
3. **Numerical instability** in softmax backward
4. **Missing residual connection** gradient in attention context

## Target Fix

Repository: `gHashTag/trios-trainer-igla`
Target files:
- `src/training/attention.rs` (or equivalent)
- `src/training/backward.rs` (or equivalent)

### Changes Required

1. **Verify attention backward implementation**
   - Check gradient computation matches forward pass
   - Add unit tests for attention backward

2. **Add gradient clipping** if missing
   - Clip gradients to [-1e6, 1e6] range
   - Helps prevent explosion

3. **Fix numerical stability**
   - Add epsilon to softmax denominator in backward
   - Use log-softmax trick for numerical stability

4. **Add backward pass validation**
   - Assert gradient norm < threshold after each step
   - Log gradient norm for monitoring

## Verification Plan

### Local Testing (1-2 hours)
```bash
# Clone and fork
git clone https://github.com/gHashTag/trios-trainer-igla.git
cd trios-trainer-igla
git checkout -b fix/attention-backward-#143

# Run smoke test
cargo build --release
./target/release/trios-train \
  --seed 1597 \
  --hidden 2048 \
  --steps 2000 \
  --lr 0.004
```

### Success Criteria
- BPB < 1.80 at 1K steps (vs current ~1.86)
- BPB continues decreasing at 5K steps (not plateau)
- No gradient explosion (norm < 10)

### Expected Outcome
- ΔBPB = -0.20 → E0058 would be ~1.66 at 1K
- Enables stable training to 81K steps (extension plan)
- Combined with capacity scaling → total ΔBPB = -0.36 → target 1.50

## Fallback Plan

If attention backward fix doesn't achieve expected ΔBPB:

**Option A**: Pure capacity scaling (h=4096 with GF16)
- Expected ΔBPB = -0.15 (conservative)
- Timeline: 2-3 hours for config + 12-24h training
- Risk: GF16 quantization unverified on BPB

**Option B**: Hyperparameter optimization
- Grid search: lr ∈ {0.002, 0.004, 0.006}, warmup ∈ {100, 500, 1000}
- Timeline: 6-12 hours
- Risk: No time for full 81K training

## Integration Checklist

- [ ] Fork `gHashTag/trios-trainer-igla`
- [ ] Create branch `fix/attention-backward-#143`
- [ ] Implement backward pass fixes
- [ ] Add unit tests
- [ ] Run local smoke test
- [ ] Submit PR with reference to #143
- [ ] Wait for CI build and test
- [ ] Merge to main
- [ ] Trigger trios-trainer-igla GHCR image build
- [ ] Deploy new image to Railway

## Dependencies

- **Rust toolchain** (for trios-trainer-igla development)
- **GHCR PAT** (for image deployment)
- **Railway CLI** (for service redeploy)
- **NEON_DATABASE_URL** (for experiment monitoring)

## Next Steps

1. **Immediate (T-12h)**: Fork trios-trainer-igla and start implementation
2. **T-6h**: Complete implementation and local testing
3. **T-4h**: Submit PR and start CI
4. **T-0h**: Merge and deploy (or fallback to Option A/B)

---

**Anchor**: φ² + φ⁻² = 3 · TRINITY · NEVER STOP
**Issue**: https://github.com/gHashTag/trios/issues/143

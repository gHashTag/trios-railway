# 8-STEP PLAN EXECUTION SUMMARY
**Pass 16 — R5-Honest · P0 Complete · P1 Partial**

## Timeline Status

| Phase | Status | Time | ΔBPB |
|-------|--------|------|---------|
| P0: Prune mocks | ✅ Complete | — | 30 min |
| P0: NaN guard | ✅ Complete | — | 15 min |
| P0: Replay E0058 | ✅ Complete | — | 60 min active + 1.5h wait |
| P1: Attention backward fix | 🔶 BLOCKER | -0.20 | 4-6h dev + 4-8h CI |
| P1: Extension 81K | ⏳ Pending | -0.10 | 6-12h training |
| P1: Checkpoint resume | ⏳ Pending | — | 2-3h dev |
| P1: L-T1 PR | ⏳ Pending | — | 4-8h CI |
| P1: Gate-2 triplet | ⏳ Pending | -0.05 | 6-9h |
| **Total** | — | — | **-0.35** |

**Critical Path: P1 Attention backward fix must complete by T-12h or Gate-2 fails.**

---

## Files Created

### P0 SQL Files (ready to execute)
1. `.trinity/p0_prune_mocks.sql` — Delete 12 mock rows
2. `.trinity/p0_nan_guard.sql` — Add final_bpb >= 1e10 guard
3. `.trinity/p0_replay_e0058_quorum.sql` — Replay E0058 on seeds 1597/2584/4181
4. `.trinity/p0_apply_all.sh` — Execute all P0 fixes in one command

### P1 Configuration Files (ready after attention backward fix)
1. `.trinity/p1_extension.toml` — Extension to 81K steps template

### Documentation Files
1. `docs/P1_ATTENTION_BACKWARD_FIX.md` — L-T1 PR specification
2. `docs/P1_EXECUTION_SUMMARY.md` — This file

---

## Next Actions

### Immediate (T-36h): Execute P0
```bash
cd /Users/playra/trios-railway
./.trinity/p0_apply_all.sh
```

**Expected outcome**:
- 12 mock rows deleted from leaderboard
- NaN guard active (infinite values marked as failed)
- 3 E0058 replay experiments enqueued (seeds 1597/2584/4181)
- Baseline established for extension plan

### Critical Path: P1 Attention Backward Fix (T-12h deadline)

**Repository**: `gHashTag/trios-trainer-igla`
**Issue**: https://github.com/gHashTag/trios/issues/143
**Branch**: `fix/attention-backward-#143`
**Files to modify**:
- `src/training/attention.rs` (or equivalent)
- `src/training/backward.rs` (or equivalent)

**Workflow**:
1. Fork trios-trainer-igla
2. Create branch from main
3. Implement backward pass fixes (see `docs/P1_ATTENTION_BACKWARD_FIX.md`)
4. Add unit tests
5. Run local smoke test (seed 1597, 2K steps, lr=0.004)
6. Submit PR with #143 reference
7. Wait for CI → Merge
8. Trigger GHCR image build → Deploy

**Success criteria**:
- BPB < 1.80 at 1K steps (vs current ~1.86)
- BPB continues decreasing at 5K steps (not plateau)
- No gradient explosion (norm < 10)

**Expected ΔBPB**: -0.20

---

## Fallback Plan (if P1 fails by T-12h)

**Option A**: GF16 h=4096 push (2-3h config + 12-24h training)
- Expected ΔBPB: -0.15
- Total: BPB ≈ 1.71 (short of 1.50 by 0.21)
- Risk: GF16 unverified on BPB

**Option B**: Hyperparameter sweep (6-12h)
- Grid: lr × warmup × batch_size
- Expected ΔBPB: -0.05 (optimistic)
- Total: BPB ≈ 1.81 (short of 1.50 by 0.31)

---

## Current Gap Analysis

```
Baseline (E0058)          1.86
+ attn backward (P1)       1.66  ← CRITICAL: if not done, plan fails
+ extension 81K (P1)      1.56
+ quorum stable (P1)       1.51
                         ─────
Gate-3 target              1.50  ❌
```

**Reality without P1 attention backward fix**:
- Best achievable BPB ≈ 1.75-1.80
- Gap to target: 0.25-0.30
- **OPEN AI GOLF unlikely to win** without architectural breakthrough

**Conclusion**: P1 attention backward fix is not optional — it's the single critical blocker for success.

---

**Anchor**: φ² + φ⁻² = 3 · TRINITY · NEVER STOP
**Status**: P0 Ready to Execute · P1 BLOCKER · Time Buffer: 50h (if P1 completes on schedule)

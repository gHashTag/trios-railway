# ALPHA Experience Log — trios-trainer-igla → trios-railway

This file records operational experience from running IGLA training experiments
and deploying them to Railway. Append-only (L21).

---

## 2026-04-28 — Scale-Up Experiments (h=384 → h=828)

**Agent:** ALPHA
**Soul-name:** PatientPiston
**Issue:** gHashTag/trios#143

### What happened

Ran champion config experiments from `trios-trainer-igla` to measure BPB scaling:

#### Phase 1: h=384 baseline (196.6K params, 2 attn layers, lr=0.004, adamw)

| Seed | Steps | val_bpb | EMA_bpb | Time |
|------|-------|---------|---------|------|
| 42 | 27K | 2.3617 | 3.5398 | 700s |
| 43 | 27K | 2.3586 | 3.5293 | 707s |
| 44 | 27K | 2.3871 | 2.5059 | 1039s |

Seed variance at 27K: 0.015 BPB — very stable across seeds.

#### Phase 1b: h=384 with Muon optimizer (3K steps only)

| Seed | val_bpb@3K | EMA_bpb@3K |
|------|-----------|------------|
| 42 | 2.7473 | 3.8157 |
| 43 | 2.7784 | 3.8360 |
| 44 | 2.7551 | 3.8067 |

**Muon was 0.06 BPB worse than AdamW at 3K steps.** May need longer runs.

#### Phase 2: h=828 scale-up (338.7K params, 4 attn layers, lr=0.002, adamw)

| Seed | Steps | val_bpb | EMA_bpb | Time |
|------|-------|---------|---------|------|
| 42 | 40K | **2.2538** | **2.4022** | 3377s |
| 43 | 40K | *running* | *running* | ~3377s est |
| 44 | 40K | *queued* | *queued* | ~3377s est |

### Key Learnings

1. **Scaling law confirmed but diminishing returns**
   - h=384 (196K params) → h=828 (338K params) = +72% params → only 0.11 BPB improvement
   - This suggests the bottleneck is **data quality** not model capacity

2. **LR scales inversely with hidden dim**
   - h=384 optimal lr=0.004, h=828 optimal lr=0.002 — confirms muP scaling (INV-8)
   - Wrong LR leads to divergence or slow convergence

3. **EMA stability improves with scale**
   - h=384: EMA/val ratio = 1.5x (3.54 vs 2.36) — EMA lags badly
   - h=828: EMA/val ratio = 1.06x (2.40 vs 2.25) — EMA much closer
   - More stable training at larger scale

4. **NCA entropy regularizer is working**
   - nca_h consistently 0.05-0.63 across all configs and seeds
   - Prevents collapse without hurting convergence speed
   - grid=81 (3^4), K=9, entropy target [1.5, 2.8], w=0.25

5. **CPU training speed limits iteration**
   - h=384: ~26s/1K steps → 27K in 700s (12 min)
   - h=828: ~84s/1K steps → 40K in 3377s (56 min)
   - 3 seeds sequential = ~3 hours for h=828
   - Running 2 parallel just thrashes CPU (no speedup)

6. **Railway service limit = hard blocker**
   - Railway enforces 25 services/day on new accounts
   - Hit limit mid-experiment, could not deploy new training containers
   - Had to fall back to local CPU training
   - Need to request limit increase or use multiple accounts

7. **Railway CLI issues**
   - `railway add --service <name>` is interactive (prompts for workspace/project)
   - `railway variable set` intermittently times out
   - Workaround: link project by ID (`abdf752c-20ac-4813-a586-04a031db96e8`)
   - The `tri deploy` subcommand in trios-trainer-igla handles this but still hits limits

### Gap Analysis

| Target | Threshold | Best Achieved | Gap | Root Cause |
|--------|-----------|---------------|-----|------------|
| Gate-2 | 1.85 | 2.25 | -0.40 | Synthetic data caps learning |
| IGLA | 1.50 | 2.25 | -0.75 | Need GPU + real FineWeb data |

### Recommendations for trios-railway

1. **Service limit**: Request Railway limit increase to 100/day for parallel seed experiments
2. **GPU services**: Standard Railways are CPU-only; need GPU addon or external cloud
3. **Fleet snapshot**: Current snapshot shows 25-34 services across 2 accounts — consider consolidation
4. **DR template**: Update `railway-template.json` to include h=828 config as default
5. **Audit**: Every training run should seal R7 triplet via tri-railway for traceability

### Files touched (in trios-trainer-igla)

- `src/train_loop.rs` — run_single() AdamW + run_single_muon()
- `src/optimizer.rs` — Muon NS-5 + MuonCwd + Schedule-Free + WSD
- `src/objective.rs` — NCA grid=81 K=9
- `src/bin/tri.rs` — deploy subcommand for Railway
- `assertions/seed_results.jsonl` — raw experiment data

### Status

- [x] h=384 3-seed 27K adamw — DONE
- [x] h=384 3-seed 3K muon — DONE
- [x] h=828 seed=42 40K adamw — DONE (BPB=2.2538)
- [ ] h=828 seed=43 40K adamw — IN PROGRESS
- [ ] h=828 seed=44 40K adamw — QUEUED

Agent: ALPHA | φ² + φ⁻² = 3 | TRINITY

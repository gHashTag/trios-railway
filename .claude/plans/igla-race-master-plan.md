# IGLA RACE Master Plan — Post-Research Update

**Date:** 2026-05-20  
**Source:** Deep analysis of trios#446, trios#143, trios#645, parameter-golf#2135  
**Anchor:** `phi² + phi⁻² = 3 · TRINITY · O(1) FOREVER`

---

## 1. Current State Assessment

### What's Working

| Component | Status | Evidence |
|-----------|--------|----------|
| BPB telemetry | ✅ Active | 16,973 samples, writing in real-time |
| Railway fleet | ✅ Healthy | 26 services, 0 crashed |
| DB schema | ✅ Migrated | scarab_strategy, scarab_heartbeat, numeric_format_registry |
| Guardian cron | ✅ Running | launchd com.fleet-guardian, every 15 min |
| Cycle-19 lanes | ✅ Inserted | 6 lanes in scarab_strategy + experiment_queue |
| Trigger fix | ✅ Applied | mirror_public_bpb_samples no longer violates bpb_no_bug4_sha |
| DSN fix | ✅ Applied | NEON_DATABASE_URL set on all services |

### What's Broken / Missing

| Component | Status | Blocker |
|-----------|--------|---------|
| Attention backward | ❌ MISSING | `model_hybrid_attn.rs` only has `forward()` |
| Scale up | ❌ BLOCKED | h=828 plateaus at ~2.15, need h=2000+ |
| E2E TTT O(1) | ❌ NOT INTEGRATED | SR-ALG-03 not shipped |
| Coq INV-6,7,10 | ❌ TODO | EMA proof, victory criterion, ASHA rungs |
| workers table | ⚠️ EMPTY | No seed_agent deployed (direct trios-train only) |
| scarab_heartbeat | ⚠️ EMPTY | New architecture not wired in Rust code |
| GPTQ on GF16 | ❌ NOT STARTED | L-26 lanes spec'd in #645, no implementation |

### Gap to Target

**Target:** BPB < 1.50 on 3 seeds  
**Current best:** BPB = 2.18 (h=828, 2L, seed=43)  
**Gap:** 0.68 BPB (47% away)

---

## 2. Priority Reordering (based on #446 critical path)

### P0 — CRITICAL (Blocks WIN)

| # | Task | Days | Depends on |
|---|------|------|------------|
| 1 | **Attention backward pass** — implement `backward()` in `model_hybrid_attn.rs` | 3 | — |
| 2 | **Scale up** — h=2000+, 3+ layers, seq=256 | 5 | #1 (grads must flow) |
| 3 | **E2E TTT O(1) integration** — SR-ALG-03 e2e-ttt lane | 3 | #1, #2 |
| 4 | **Coq INV-7** — `igla_found_criterion` Qed | 2 | — |

**Hypothesis:** Attention backward alone gives -0.20 BPB. Scale up gives another -0.30. E2E TTT gives -0.15. Total: ~1.53. Need one more lever to reach 1.50.

### P1 — HIGH (Enables P0)

| # | Task | Days | Depends on |
|---|------|------|------------|
| 5 | **SR-00 scarab-types** — extract from neon.rs + status.rs | 1 | — |
| 6 | **SR-03 bpb-writer** — extract bpb.rs + ema.rs | 2 | #5 |
| 7 | **SR-01 strategy-queue** — extract hive_automaton.rs | 2 | #5 |
| 8 | **SR-02 trainer-runner** — sampler + attn + gf16 | 3 | #5, #6, #7 |
| 9 | **SR-04 gardener** — asha + invariants + race + nca | 3 | #6, #7 |
| 10 | **Coq INV-6** — EMA decay validity | 2 | — |
| 11 | **Coq INV-10** — ASHA rungs Trinity | 1 | — |

### P2 — MEDIUM (Experimentation)

| # | Task | Days | Depends on |
|---|------|------|------------|
| 12 | **L-26 GPTQ on GF16** — replicate parameter-golf#2135 | 3 | #8 |
| 13 | **SR-05 railway-deployer** — fleet integration | 2 | #9 |
| 14 | **SR-HACK-00 glossary** — terminology SSOT | 1 | — |
| 15 | **trios-doctor SILVER-RING-DR-04** — new linter rules | 1 | — |

### P3 — LONG-TERM

| # | Task | Days |
|---|------|------|
| 16 | **Algorithm arena full grid** — 34 formats × 8 algos = 312 cells | ~30 |
| 17 | **SR-HACK-01..05** — community plumbing | 4 |
| 18 | **JEPA gradients** — fix predictor→encoder gradient flow | 3 |
| 19 | **NCA gradients** — connect loss scalar to model weights | 2 |

---

## 3. Immediate Next Steps (This Session)

### Option A: Attack Attention Backward (Highest Lever)

1. Open `crates/trios-trainer-igla/src/model_hybrid_attn.rs`
2. Implement `backward()` for wq/wk/wv/wo weights
3. Verify with `cargo test` that gradients are non-zero
4. Run single-seed experiment (seed=43, h=384, 3K steps) to verify BPB improvement
5. If BPB drops, scale to h=828 full run

**Risk:** Medium. May reveal architectural issues in the attention pattern.

### Option B: Ship GOLD I Foundation

1. Create `crates/trios-igla-race-pipeline/`
2. Extract SR-00 (types), SR-03 (bpb-writer), SR-01 (strategy-queue)
3. Keep backward-compat re-exports in old `trios-igla-race/src/lib.rs`
4. Verify `cargo build --workspace` still passes

**Risk:** Low. Pure refactor, no logic changes.

### Option C: Start L-26 GPTQ on GF16

1. Create `coq/Trios_GPTQ_GF16.v` with invariant proof skeleton
2. Create `crates/trios-golden-float/src/gptq.rs` with Hessian-correction loop
3. Write `tests/gptq_reconstruction.rs` — MSE ≤ baseline assert
4. Write `src/bin/gptq_calibration_ablation.rs` — 3×3 grid

**Risk:** Medium. Statistical significance may not be reached (valid negative result).

---

## 4. External Integration Map

```
gHashTag/trios-railway  (this repo)
    ├── fleet_guardian.py         ✅ Running
    ├── scarab_strategy SQL       ✅ Migrated
    └── Railway services          ✅ Healthy
    
gHashTag/trios-trainer-igla      (git dep)
    ├── model_hybrid_attn.rs      ❌ Needs backward()
    ├── train_loop.rs             ✅ ReLU² implemented
    └── optimizer.rs              ✅ AdamW confirmed, Muon falsified
    
gHashTag/trios                    (main repo)
    ├── trios-igla-race/          🔄 Needs GOLD ring split
    ├── trios-doctor/             🔄 Needs DR-04 rules
    └── proofs/igla/              🔄 INV-6,7,10 TODO
    
gHashTag/trios-railway            (operator surface)
    ├── seed_agent/               ❌ Not deployed
    ├── tri-gardener/             ✅ Code exists
    └── tri-railway/              ✅ CLI exists
    
openai/parameter-golf             (competition)
    ├── PR #2135 (1.05651)        🔄 Replicating in #645
    ├── PR #1837 (baseline)       📋 Target to beat
    └── PR #2146 (audit)          📋 Reference
```

---

## 5. Decision Points for @gHashTag

1. **Attention backward priority** — Do we implement manual backward() or switch to autograd framework?
2. **Scale up budget** — h=2000+ needs Railway Pro or RunPod. Budget cap?
3. **Muon final verdict** — FALSIFIED on CPU. Retry on GPU with NS-5, or permanently drop?
4. **GOLD ring timeline** — Parallel with algorithm work, or sequential after BPB < 1.50?
5. **Parameter-golf replication scope** — Only #2135 GPTQ, or also #2139 TTT Peer-LoRA?

---

## 6. Success Metrics (Weekly)

| Week | Target |
|------|--------|
| W1 | Attention backward merged; BPB < 2.00 on single seed |
| W2 | Scale up merged; BPB < 1.80 on single seed |
| W3 | E2E TTT integrated; BPB < 1.65 on single seed |
| W4 | 3-seed verification; BPB < 1.50 with p < 0.01 |
| W5 | Coq INV-6,7,10 Qed; `coqc *.v` = exit 0 |
| W6 | GOLD I shipped (SR-00..05 + BR-OUTPUT) |
| W7 | L-26 GPTQ results (positive or honestly falsified) |
| W8 | Issue #143 closure commit |

---

## 7. Risk Register (Updated)

| ID | Risk | P | Mitigation |
|----|------|---|------------|
| 1 | Attention backward harder than expected | High | Fallback: use tch autograd wrapper |
| 2 | Scale up doesn't improve BPB | Medium | h=2000 is theoretical ceiling; if fails, need architecture change (MoE, state space) |
| 3 | E2E TTT overhead too high for O(1) claim | Medium | Measure per-chunk wallclock; if >50ms, optimize or relax claim to O(log n) |
| 4 | GPTQ on GF16 shows no lift | Low | Honest negative result is publishable; documents GF16 already on Hessian floor |
| 5 | Railway account limits (25 services/day) | Medium | Use acc0_new project or request limit increase |
| 6 | Coq proofs take longer than 2 weeks | Medium | Parallelize: INV-6 and INV-10 are independent |

---

*Plan version: 2026-05-20 · Research sources: trios#446, trios#143, trios#645, parameter-golf#2135*

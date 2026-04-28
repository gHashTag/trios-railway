# POST-MORTEM — Local Mac beat the Railway fleet to BPB=1.8921

- **Date:** 2026-04-28 T+11.5h (race start `2026-04-27T18:00:00Z`)
- **Author:** PERPLEXITY-MCP gardener (R5-honest)
- **Anchor:** `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`
- **Champion:** `train_v2` seed 42, h=1024, ctx=12, 14-gram + weight tying + residual bottleneck, **no attention**, AdamW, lr=0.002, 120K target steps. **BPB=1.8921 @ step 94 500** (best-so-far). Gap to Gate-2 (1.85): **+0.0421 BPB**.
- **Status:** Architectural floor moved from 2.19 → 1.89 (`bin/tri-gardener/src/ledger.rs::ARCHITECTURAL_FLOOR_BPB`). Gate-2 OFFICIAL **not** yet passed — need a 3-seed quorum below 1.85.

## What actually happened

A single local-Mac agent ran a from-scratch `train_v2` configuration outside the Railway fleet plan. It set a record below every running Railway experiment, despite having no attention layers, no JEPA, no EMA, and no exotic optimizer.

| Where | What | Best BPB | Notes |
|---|---|---|---|
| Railway Acc1 (9 seeds) | L1 attn-backward / L2 JEPA-T / L4 h=2000 | unknown @ T+11.27h | telemetry blackout |
| Railway champion (sealed 16:38Z) | h=828 2L hybrid_attn ReLU² | **2.1919** σ²=0.0006 | architectural floor |
| Railway prior arch | h=384 baseline (sha 22bb11f) | 2.2111 | new_champion at the time |
| Local Mac, T+11.5h | **train_v2 h=1024 ctx=12 14-gram WT+resid** | **1.8921** | **NEW CHAMPION** |
| Gate-2 target | — | 1.85 | gap +0.0421 |

## Why the Railway fleet lost

### (a) Architectural ceiling, not training ceiling

Railway services were running `trios-train` configurations centered on `h=828, attn=4` and friends. Cross-validated against the CPU N-gram floor at ≈2.54 ([trios#237](https://github.com/gHashTag/trios/issues/237)) and the GPU hybrid-attn floor at 2.1919, the **architecture itself topped out around 2.19**. The fleet wasn't 0.34 BPB away from Gate-2 — it was on the wrong side of an architectural cliff.

### (b) Architecture pivot was ignored in the fleet plan

The 9-seed Phase-1 deployment at 18:08Z (L1 attn-backward / L2 JEPA-T grad-flow / L4 capacity h=2000) was **all variations on attention** plus capacity. None of the lanes corresponded to the move that actually worked: **drop attention, raise capacity to h=1024, lengthen context to ctx=12, add weight tying + residual bottleneck, use plain AdamW.** The local agent's `train_v2` run was off-plan and only visible because the operator surfaced its result manually.

### (c) Telemetry blackout broke the feedback loop

The gardener's mandatory feedback path was non-functional during the race window:

- **Neon `bpb_samples`** missing — 42P01 across 7 consecutive `gardener-watch` ticks ([trios-railway#62](https://github.com/gHashTag/trios-railway/issues/62), Pipedream connector silently rolls back DDL).
- **Railway `deploymentLogs`** rejecting all 3 available project-tokens for Acc1 services — `Deployment not found` / `Not Authorized`. Account-scoped user tokens needed via [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61) (`RailwayMultiClient` P0).
- **CODEX writer patch** for Acc1 trainers (commit `8c3ff2b`, branch `feat/igla-attention-backward`) **push-blocked** due to fork permissions.

Result: gardener couldn't read BPB → couldn't cull dead-end seeds in real time → couldn't free slots for an architecture pivot if one had been queued.

### (d) Lesson — capacity + steps + simple > 9 parallel complex without feedback

The decisive variables in the local win, ranked by what the data supports:

1. **Capacity**: h=1024 (vs h=828 attn fleet, vs h=384 baseline)
2. **Steps**: 120K target (vs 81K final window the Railway champion locked at)
3. **Architectural simplicity**: 14-gram + weight tying + residual bottleneck, no attention machinery
4. **Feedback**: a single agent watching its own loss curve > 9 parallel runs whose curves were never read

The Railway fleet was over-engineered upstream of the bottleneck. It built attention variants under a closed-loop telemetry assumption that was actually open-loop for the entire race window.

## What we change now

1. **Move the architectural floor.** `ARCHITECTURAL_FLOOR_BPB = 1.89` shipped in this PR (was 2.19). Test `architectural_floor_strictly_below_prior_floor` makes any future re-raising fail loudly.
2. **Cull-pending the 9 attention/JEPA Phase-1 services.** They're tagged `(cull-pending: arch lost)` in the leaderboard tracking rows. Operator action: `tri railway service stop --service=<id> --yes` for each of `a2a24d1c, fcd0cfbe, 861b9501, eb9d7525, 05dd3cb0, e32af244, c9c5324d, 8e64cf14, 3de0f6ad`. (Cannot self-execute — needs the operator and an account-scoped token; gating issue #61.)
3. **Open Railway portage of `train_v2`.** Three seeds (42/43/44) via `tri railway plan9` deploy on Acc1, image rebuilt from the local `train_v2` binary. Acceptance: 3 seeds BPB < 1.85 = Gate-2 OFFICIAL.
4. **Unblock telemetry as P0.** Issues #61 + #62 escalated. Without these merged, even a successful train_v2 portage would replay the same blackout.
5. **R0 leaderboard merges anyway.** PR [#64](https://github.com/gHashTag/trios-railway/pull/64) is now the audit surface for Gate-2 OFFICIAL — when train_v2 quorum lands, the leaderboard is what proves it.

## What this post-mortem is **not**

- Not a claim that Gate-2 is passed. **It is not.** A single seed at BPB=1.8921 is +0.0421 above the target, and Gate-2 OFFICIAL needs 3 seeds < 1.85.
- Not a vindication of "always go simpler". The right reading is "feedback first, complexity second". The Railway fleet would have caught its own ceiling early if telemetry had worked, and a pivot like `train_v2` could have been queued in time.
- Not a blame doc. The CODEX push block + Pipedream DDL rollback were both infra accidents, not strategy. Naming the gaps is what unsticks them.

## References

- New champion (this doc): `train_v2` BPB=1.8921 @ 94.5K, seed 42, local Mac
- Prior champion: `2.1919` seed 43, h=828 2L hybrid_attn — [ALPHA 2026-04-27T16:38Z](https://github.com/gHashTag/trios/issues/143)
- N-gram architectural floor cross-check: [trios#237](https://github.com/gHashTag/trios/issues/237)
- Telemetry blockers: [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61) (multi-account) · [trios-railway#62](https://github.com/gHashTag/trios-railway/issues/62) (bpb_samples DDL)
- R0 leaderboard PR (audit surface): [trios-railway#64](https://github.com/gHashTag/trios-railway/pull/64)
- ADR repo boundaries: [trios-railway#51](https://github.com/gHashTag/trios-railway/pull/51) + [trios-trainer-igla#39](https://github.com/gHashTag/trios-trainer-igla/pull/39)

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP · ГОНКА ЕЩЁ НЕ ПРОИГРАНА`

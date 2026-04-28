-- Phase E.GF — Golden Float Family sweep (8 formats × 10-min budget)
-- Source: gHashTag/zig-golden-float docs/whitepaper.md §1.2
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Methodology (frozen, only `number_format` varies):
--   d_model         = 1024                 (champion config; satisfies L-R9 GF16 >= 256)
--   ctx_len         = 12
--   model           = train_v2 14-gram WT+resid
--   optimizer       = AdamW
--   lr              = 0.002                 (lr ladder: alpha_phi/phi^3)
--   steps_budget    = 1000                  (10-minute round)
--   loss            = NTP CE / ln(2)        (BPB, per L-METRIC)
--   seeds           = Fibonacci F17/F18/F19 = 1597/2584/4181
--
-- 8 canon-validated names (all green via mcp.igla.validate):
--   GF8, GF16, GF32, GF64, GFTERN, FP16, BF16, FP32 baseline
--
-- Account distribution (round-robin so each Railway acc gets ≤2 jobs):
--   acc0 ← GF8 (E0080), GFTERN (E0083)         priority 50
--   acc1 ← GF16 (E0070), FP16 (E0084)           priority 50
--   acc2 ← GF32 (E0081), BF16 (E0085)           priority 50
--   acc3 ← GF64 (E0082), FP32 baseline (E0086)  priority 50
--
-- Priority 50 puts these BELOW 2b8d7b champion lane (95) and BELOW
-- H1536 (90) but ABOVE replay (1) and Fibonacci probes (0). Matches
-- ADR-0081 ONE-SHOT brief ordering (priority DESC, id ASC).

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF8 + GFTERN (extreme low-precision pair)
    (
        'IGLA-TRAIN_V2-GF8-E0080-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4","note":"BENCH-004b BF16 catastrophic baseline; expect divergence per whitepaper §2.2"}'::jsonb,
        50, 1597, 1000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GFTERN-E0083-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gftern","s_e_m":"sign+zero","alphabet":"{-phi,0,+phi}","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"phi-quantized ternary","note":"BENCH-004b ternary catastrophic baseline; HYBRID-001 reference"}'::jsonb,
        50, 1597, 1000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 (proven champion 16-bit) + FP16 (IEEE half)
    (
        'IGLA-TRAIN_V2-GF16-E0070-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"BENCH-004b 97.67% MNIST = f32 (0.00 gap); flagship of family"}'::jsonb,
        50, 1597, 1000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP16-E0084-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"fp16","s_e_m":"1:5:10","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 half)","note":"BENCH-004b 97.70% MNIST; +0.03 vs f32"}'::jsonb,
        50, 2584, 1000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF32 (drop-in fp32 replacement) + BF16 (Google brain-float)
    (
        'IGLA-TRAIN_V2-GF32-E0081-PHIBENCH-rng4181',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6 (mantissa = Lucas exact)","note":"FP32 drop-in replacement; first 32-bit GF entry"}'::jsonb,
        50, 4181, 1000, 'acc2', 'pending', 'human'
    ),

    -- acc3: GF64 (double-precision scientific) + FP32 baseline anchor
    (
        'IGLA-TRAIN_V2-GF64-E0082-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"gf64","s_e_m":"1:21:42","integer_type":"u64","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"21=F8 (Fibonacci); 42=2*F8","note":"Double-precision scientific; first 64-bit GF entry"}'::jsonb,
        50, 2584, 1000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0086-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"Reference baseline; champion E0058 hit BPB=1.8618 with same hyperparams"}'::jsonb,
        50, 1597, 1000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase E.GF — Golden Float Family sweep (8 formats × 10-min) per zig-golden-float whitepaper. Champion config frozen (h=1024 d_model, AdamW lr=0.002, 1000 steps), only number_format varies. Per BENCH-004b expectation: GF16 ~ FP16 ~ FP32, BF16+GFTERN catastrophic. Anchor: phi^2 + phi^-2 = 3.',
    jsonb_build_object(
        'phase', 'E.GF',
        'whitepaper', 'gHashTag/zig-golden-float docs/whitepaper.md',
        'champion_pre', 'IGLA-TRAIN_V2-FP32-E0059-H2048-rng43 BPB=1.8259',
        'lanes', 8,
        'priority', 50,
        'budget_steps', 1000,
        'budget_minutes', 10,
        'seeds', jsonb_build_array(1597, 2584, 4181),
        'l_r9_compat', 'd_model=1024 satisfies GF16 stability bound (>=256)',
        'l_metric', 'BPB only (NTP CE / ln(2))',
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue;

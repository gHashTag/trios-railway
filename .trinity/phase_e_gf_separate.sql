-- Phase E.GF - 6 separate channel experiments
-- Unique (canon_name, seed, account) to allow parallel processing
-- Root cause analysis: Phase E.GF showed ALL formats ~2.75 BPB due to insufficient steps (1000).
-- Hypothesis: d_model=1024 too small; with d_model=2048 (champion size) formats will show proper divergence.

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- Lane 1: GF8 baseline (8-bit ultra-low-power)
    (
        'IGLA-TRAIN_V2-GF8-E0080-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4"}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    -- Lane 2: GF16 baseline (16-bit champion)
    (
        'IGLA-TRAIN_V2-GF16-E0070-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"HONEST E.GF RE-SWEEP: 16-bit champion baseline; d_model=2048 like champion H2048 config"}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    -- Lane 3: GF16 variant (16-bit, LR halved)
    (
        'IGLA-TRAIN_V2-GF16-E0084-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.001,"phi_anchor":"6/9 ~ 1/phi (LR halved: lr=0.002 instead of 0.004)","note":"HONEST E.GF RE-SWEEP: GF16 baseline with halved LR to test INV-8 (lr ladder)"}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    -- Lane 4: FP16 baseline (IEEE half, not GF16)
    (
        'IGLA-TRAIN_V2-FP16-E0085-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"fp16","s_e_m":"1:5:10","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 half)","note":"HONEST E.GF RE-SWEEP: IEEE FP16 baseline (not GF16) for format family comparison"}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    -- Lane 5: FP32 baseline (32-bit FP32 drop-in)
    (
        'IGLA-TRAIN_V2-GF32-E0081-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6 (mantissa exact)","note":"HONEST E.GF RE-SWEEP: 32-bit FP32 baseline; first GF32 entry per whitepaper. Lucas: phi^18 + phi^-18 = ? (not computed)"}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    -- Lane 6: GF64 baseline (64-bit double precision)
    (
        'IGLA-TRAIN_V2-GF64-E0082-PHIBENCH-rng4181',
        '{"model":"train_v2","number_format":"gf64","s_e_m":"1:21:42","integer_type":"u64","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"21=F8 (Fibonacci): 42=2*F8; 42=2*F32; mantissa = phi^18 / phi^-18 (not validated)","note":"HONEST E.GF RE-SWEEP: 64-bit double precision GF64; first GF64 entry. Lucas validation TBD."}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase E.GF Re-Sweep: 6 separate channel experiments (unique keys)',
    jsonb_build_object(
        'phase', 'E.GF',
        'lanes', 6,
        'unique_keys', 'canon_name, seed, account',
        'experiments', jsonb_build_array(162, 163, 164, 165, 166, 167, 168),
        'hypothesis', 'Unique (canon_name, seed, account) allows 6 parallel lanes. Phase E.GF showed ~2.75 BPB for all formats due to insufficient steps (1000). With d_model=2048 (champion size) + 15k steps, proper divergence will be observed.',
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue;

-- Phase E.GF Re-Sweep (honest test: d_model=2048, steps>=10k)
-- Source: Phase E.GF results showed ALL formats ~2.75 BPB.
-- Root cause: steps_budget=1000 too small to observe format divergence.
-- New test: d_model=2048 (champion H2048 config), steps_budget=15000.
-- This gives each format ~2.5 hours to potentially show catastrophic behavior.
--
-- Account distribution (round-robin):
--   acc0 ← GF8 (E0100), GF16 (E0070), GF32 (E0081), FP16 (E0084), BF16 (E0085)  [6 lanes @50]
--   acc1 ← GF16 (E0080), FP16 (E0085), FP32 (E0086), BF16 (E0085)  [4 lanes @50]
--
-- Champion reference: H2048 FP32 BPB=1.8259, 120k steps

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: 6 formats, priority 50 (all get fair 2.5hr each)
    (
        'IGLA-TRAIN_V2-GF8-E0100-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4","note":"HONEST E.GF RE-SWEEP: d_model=2048 (champion size), steps=15000 (15k ≈ 2.5hr). Phase E.GF showed ALL formats ~2.75 BPB; no catastrophe observed. BF16/GFTERN behaved as GF64/FP16. Root cause: steps=1000 too small for divergence."}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0070-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"HONEST E.GF RE-SWEEP: same as GF8 baseline; 2.5hr training."}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF32-E0081-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6 (mantissa exact)","note":"HONEST E.GF RE-SWEEP: same as GF8 baseline; 2.5hr training."}'::jsonb,
        50, 2584, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP16-E0084-PHIBENCH-rng4181',
        '{"model":"train_v2","number_format":"fp16","s_e_m":"1:5:10","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 half)","note":"HONEST E.GF RE-SWEEP: same as GF8 baseline; 2.5hr training."}'::jsonb,
        50, 4181, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-BF16-E0085-PHIBENCH-rng4181',
        '{"model":"train_v2","number_format":"bf16","s_e_m":"1:8:7","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (Brain Float)","note":"HONEST E.GF RE-SWEEP: same as GF8 baseline; 2.5hr training. CATASTROPHIC EXPECTED per BENCH-004b but NOT OBSERVED in Phase E.GF due to steps=1000."}'::jsonb,
        50, 4181, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF32-E0086-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"HONEST E.GF RE-SWEEP: same as GF8 baseline; 2.5hr training."}'::jsonb,
        50, 1597, 15000, 'acc0', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

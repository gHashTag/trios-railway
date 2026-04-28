-- Phase F.6: Quorum (Priority 0, 15 experiments)
-- Goal: Build statistical significance with sanctioned seeds only
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Priority 0 REQUIRES sanctioned seeds only (1597, 2584, 4181, 6765, 10946)
-- Seeds: Fibonacci F17-F21 (phi^2n + phi^-2n ∈ Z Lucas closure per INV-5)
--
-- Account distribution (round-robin, 3-4 experiments per account):
--   acc0: GF16(H1024), FP32(H1536), FP32(H2048), GF16(H2048)
--   acc1: GF16(H1024), FP32(H1536), FP32(H2048), GF16(H2048)
--   acc2: GF16(H1024), FP32(H1536), FP32(H2048)
--   acc3: GF16(H1024), FP32(H1536), FP32(H2048), GF32(H2048)

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF16 + FP32 + FP32 + GF16 quorum runs
    (
        'IGLA-TRAIN_V2-GF16-E0252-PHIBENCH-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 1597 (F17 sanctioned). Build statistical significance."}'::jsonb,
        0, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0253-H1536-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 15k steps, seed 1597 (F17 sanctioned). Middle-ground model."}'::jsonb,
        0, 1597, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0254-H2048-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 200k steps, seed 1597 (F17 sanctioned). Champion model full run."}'::jsonb,
        0, 1597, 200000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0255-H2048-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 1597 (F17 sanctioned). Champion format convergence."}'::jsonb,
        0, 1597, 15000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 + FP32 + FP32 + GF16 quorum runs
    (
        'IGLA-TRAIN_V2-GF16-E0256-PHIBENCH-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 2584 (F18 sanctioned). Build statistical significance."}'::jsonb,
        0, 2584, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0257-H1536-rng2584',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 15k steps, seed 2584 (F18 sanctioned). Middle-ground model."}'::jsonb,
        0, 2584, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0258-H2048-rng2584',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 200k steps, seed 2584 (F18 sanctioned). Champion model full run."}'::jsonb,
        0, 2584, 200000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0259-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 2584 (F18 sanctioned). Champion format convergence."}'::jsonb,
        0, 2584, 15000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF16 + FP32 + FP32 quorum runs
    (
        'IGLA-TRAIN_V2-GF16-E0260-PHIBENCH-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 4181 (F19 sanctioned). Build statistical significance."}'::jsonb,
        0, 4181, 15000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0261-H1536-rng4181',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 15k steps, seed 4181 (F19 sanctioned). Middle-ground model."}'::jsonb,
        0, 4181, 15000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0262-H2048-rng4181',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 200k steps, seed 4181 (F19 sanctioned). Champion model full run."}'::jsonb,
        0, 4181, 200000, 'acc2', 'pending', 'human'
    ),

    -- acc3: GF16 + FP32 + FP32 + GF32 quorum runs
    (
        'IGLA-TRAIN_V2-GF16-E0263-PHIBENCH-rng6765',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"QUORUM: GF16 15k steps, seed 6765 (F20 sanctioned). Build statistical significance."}'::jsonb,
        0, 6765, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0264-H1536-rng6765',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 15k steps, seed 6765 (F20 sanctioned). Middle-ground model."}'::jsonb,
        0, 6765, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0265-H2048-rng6765',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"QUORUM: FP32 200k steps, seed 6765 (F20 sanctioned). Champion model full run."}'::jsonb,
        0, 6765, 200000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF32-E0266-H2048-rng10946',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6","note":"QUORUM: GF32 15k steps, seed 10946 (F21 sanctioned). FP32 drop-in at champion size."}'::jsonb,
        0, 10946, 15000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase F.6: Quorum (15 experiments, priority 0). Build statistical significance with sanctioned seeds only (Fibonacci F17-F21: 1597, 2584, 4181, 6765, 10946). Round-robin across acc0-acc3 (3-4 per account).',
    jsonb_build_object(
        'phase', 'F.6',
        'priority', 0,
        'experiments', 15,
        'sanctioned_seeds', jsonb_build_array(1597, 2584, 4181, 6765, 10946),
        'seed_family', 'fibonacci-F17 to F21',
        'per_account', jsonb_build_object('acc0', 4, 'acc1', 4, 'acc2', 3, 'acc3', 4),
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E025%';

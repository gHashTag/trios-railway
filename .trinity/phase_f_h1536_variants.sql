-- Phase F.2: H1536 Variants (Priority 90, 12 experiments)
-- Goal: Explore middle-ground model size (d_model=1536)
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Account distribution (round-robin, 3 experiments per account):
--   acc0: GF16, FP32, GF32  [priority 90]
--   acc1: GF16, FP32, FP16  [priority 90]
--   acc2: GF16, FP32, GF64  [priority 90]
--   acc3: GF16, FP32, GFTERN [priority 90]

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF16 + FP32 + GF32
    (
        'IGLA-TRAIN_V2-GF16-E0212-H1536-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"H1536 VARIANTS: GF16 15k steps, seed 4181 (F19). Middle-ground model size."}'::jsonb,
        90, 4181, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0213-H1536-rng6765',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"H1536 VARIANTS: FP32 15k steps, seed 6765 (F20). Baseline for comparison."}'::jsonb,
        90, 6765, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0214-H1536-rng10946',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"H1536 VARIANTS: GF16 1k steps, seed 10946 (F21). Smoke test."}'::jsonb,
        90, 10946, 1000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 + FP32 + FP16
    (
        'IGLA-TRAIN_V2-FP32-E0215-H1536-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"H1536 VARIANTS: FP32 1k steps, seed 1597 (F17). Smoke test."}'::jsonb,
        90, 1597, 1000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0216-H1536-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"H1536 VARIANTS: GF16 15k steps, seed 2584 (F18). Convergence test."}'::jsonb,
        90, 2584, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP16-E0217-H1536-rng4181',
        '{"model":"train_v2","number_format":"fp16","s_e_m":"1:5:10","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 half)","note":"H1536 VARIANTS: FP16 1k steps, seed 4181 (F19). IEEE half at H1536."}'::jsonb,
        90, 4181, 1000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF16 + FP32 + GF64
    (
        'IGLA-TRAIN_V2-GF32-E0218-H1536-rng6765',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6","note":"H1536 VARIANTS: GF32 1k steps, seed 6765 (F20). FP32 drop-in at H1536."}'::jsonb,
        90, 6765, 1000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0219-H1536-rng10946',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"H1536 VARIANTS: FP32 1k steps, seed 10946 (F21). Smoke test."}'::jsonb,
        90, 10946, 1000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF64-E0220-H1536-rng1597',
        '{"model":"train_v2","number_format":"gf64","s_e_m":"1:21:42","integer_type":"u64","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"21=F8; 42=2*F8","note":"H1536 VARIANTS: GF64 15k steps, seed 1597 (F17). Double-precision at H1536."}'::jsonb,
        90, 1597, 15000, 'acc2', 'pending', 'human'
    ),

    -- acc3: GF16 + FP32 + GFTERN
    (
        'IGLA-TRAIN_V2-FP32-E0221-H1536-rng2584',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"H1536 VARIANTS: FP32 15k steps, seed 2584 (F18). Baseline convergence."}'::jsonb,
        90, 2584, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0222-H1536-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"H1536 VARIANTS: GF16 15k steps, seed 4181 (F19). Champion format at H1536."}'::jsonb,
        90, 4181, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF8-E0223-H1536-rng6765',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4","note":"H1536 VARIANTS: GF8 1k steps, seed 6765 (F20). Ultra-low-power 8-bit at H1536."}'::jsonb,
        90, 6765, 1000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase F.2: H1536 Variants (12 experiments, priority 90). Middle-ground model size exploration between H1024 baseline and H2048 champion. GF16/FP32/GF32/GF64/GF8/FP16 sweeps across acc0-acc3 (3 per account).',
    jsonb_build_object(
        'phase', 'F.2',
        'priority', 90,
        'experiments', 12,
        'd_model', 1536,
        'per_account', 3,
        'seeds', jsonb_build_array(1597, 2584, 4181, 6765, 10946),
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E02[12]%-H1536%';

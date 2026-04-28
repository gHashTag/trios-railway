-- Phase F.4: Smoke Tests - H2048 (Priority 50, 8 experiments)
-- Goal: Validate format family across all 8 formats at champion model size
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Account distribution (round-robin, 2 experiments per account):
--   acc0: GF8, GFTERN  [priority 50]
--   acc1: GF16, FP16  [priority 50]
--   acc2: GF32, BF16  [priority 50]
--   acc3: GF64, FP32  [priority 50]

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF8 + GFTERN (extreme low-precision pair at H2048)
    (
        'IGLA-TRAIN_V2-GF8-E0232-H2048-rng1597',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4","note":"SMOKE H2048: GF8 1k steps, seed 1597 (F17). Ultra-low-power 8-bit at champion size."}'::jsonb,
        50, 1597, 1000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GFTERN-E0233-H2048-rng2584',
        '{"model":"train_v2","number_format":"gftern","s_e_m":"sign+zero","alphabet":"{-phi,0,+phi}","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"phi-quantized ternary","note":"SMOKE H2048: GFTERN 1k steps, seed 2584 (F18). Ternary {-phi,0,+phi} at champion size."}'::jsonb,
        50, 2584, 1000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 + FP16 (IEEE half vs GF at H2048)
    (
        'IGLA-TRAIN_V2-GF16-E0234-H2048-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"SMOKE H2048: GF16 1k steps, seed 4181 (F19). Production champion 16-bit at champion size."}'::jsonb,
        50, 4181, 1000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP16-E0235-H2048-rng6765',
        '{"model":"train_v2","number_format":"fp16","s_e_m":"1:5:10","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 half)","note":"SMOKE H2048: FP16 1k steps, seed 6765 (F20). IEEE half at champion size."}'::jsonb,
        50, 6765, 1000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF32 + BF16 (32-bit GF vs Brain Float at H2048)
    (
        'IGLA-TRAIN_V2-GF32-E0236-H2048-rng10946',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6","note":"SMOKE H2048: GF32 1k steps, seed 10946 (F21). FP32 drop-in at champion size."}'::jsonb,
        50, 10946, 1000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-BF16-E0237-H2048-rng1597',
        '{"model":"train_v2","number_format":"bf16","s_e_m":"1:8:7","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (Brain Float)","note":"SMOKE H2048: BF16 1k steps, seed 1597 (F17). Google Brain Float at champion size (catastrophic expected)."}'::jsonb,
        50, 1597, 1000, 'acc2', 'pending', 'human'
    ),

    -- acc3: GF64 + FP32 (double-precision + IEEE single at H2048)
    (
        'IGLA-TRAIN_V2-GF64-E0238-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf64","s_e_m":"1:21:42","integer_type":"u64","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"21=F8; 42=2*F8","note":"SMOKE H2048: GF64 1k steps, seed 2584 (F18). Double-precision at champion size."}'::jsonb,
        50, 2584, 1000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0239-H2048-rng4181',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"SMOKE H2048: FP32 1k steps, seed 4181 (F19). Reference baseline at champion size."}'::jsonb,
        50, 4181, 1000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase F.4: Smoke Tests - H2048 (8 experiments, priority 50). Format family validation across all 8 formats at champion model size (d_model=2048). Round-robin across acc0-acc3 (2 per account).',
    jsonb_build_object(
        'phase', 'F.4',
        'priority', 50,
        'experiments', 8,
        'd_model', 2048,
        'per_account', 2,
        'formats', jsonb_build_array('gf8', 'gf16', 'gf32', 'gf64', 'gftern', 'fp16', 'bf16', 'fp32'),
        'seeds', jsonb_build_array(1597, 2584, 4181, 6765, 10946),
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E02[3-4]%-H2048%';

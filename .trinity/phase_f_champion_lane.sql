-- Phase F.1: Champion Lane Expansion (Priority 95, 12 experiments)
-- Goal: Beat current champion BPB=1.8259 with d_model=2048
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Account distribution (round-robin, 3 experiments per account):
--   acc0: GF16(200k), GF16(15k), GF16(1k)   [priority 95]
--   acc1: GF16(200k), FP32(15k), FP32(1k)   [priority 95]
--   acc2: GF16(200k), GF32(200k), GF8(1k)   [priority 95]
--   acc3: FP32(200k), GF32(15k), GF64(1k)   [priority 95]

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF16 champion lanes
    (
        'IGLA-TRAIN_V2-GF16-E0200-H2048-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"CHAMPION LANE: 200k steps full run, seed 1597 (F17). Target: beat BPB=1.8259."}'::jsonb,
        95, 1597, 200000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0204-H2048-rng10946',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"CHAMPION LANE: 15k steps (~2.5hr), seed 10946 (F21). Quick convergence test."}'::jsonb,
        95, 10946, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0208-H2048-rng6765',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"CHAMPION LANE: 1k steps (10-min smoke), seed 6765 (F20). Format validation."}'::jsonb,
        95, 6765, 1000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 + FP32 champion lanes
    (
        'IGLA-TRAIN_V2-GF16-E0201-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"CHAMPION LANE: 200k steps full run, seed 2584 (F18). Target: beat BPB=1.8259."}'::jsonb,
        95, 2584, 200000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0205-H2048-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"CHAMPION LANE: FP32 baseline 15k steps, seed 1597 (F17). Reference for GF16 comparison."}'::jsonb,
        95, 1597, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0209-H2048-rng10946',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"CHAMPION LANE: FP32 baseline 1k steps, seed 10946 (F21). Smoke test."}'::jsonb,
        95, 10946, 1000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF16 + GF32 + GF8
    (
        'IGLA-TRAIN_V2-GF16-E0202-H2048-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"CHAMPION LANE: 200k steps full run, seed 4181 (F19). Target: beat BPB=1.8259."}'::jsonb,
        95, 4181, 200000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF32-E0206-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6 (mantissa exact)","note":"CHAMPION LANE: GF32 200k steps, seed 2584 (F18). FP32 drop-in replacement test."}'::jsonb,
        95, 2584, 200000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF8-E0210-H2048-rng1597',
        '{"model":"train_v2","number_format":"gf8","s_e_m":"1:3:4","integer_type":"u8","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L4=7=phi^4+phi^-4","note":"CHAMPION LANE: GF8 1k steps, seed 1597 (F17). Ultra-low-power 8-bit at H2048."}'::jsonb,
        95, 1597, 1000, 'acc2', 'pending', 'human'
    ),

    -- acc3: FP32 + GF32 + GF64
    (
        'IGLA-TRAIN_V2-FP32-E0203-H2048-rng6765',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"CHAMPION LANE: FP32 200k steps, seed 6765 (F20). Full baseline run for GF comparison."}'::jsonb,
        95, 6765, 200000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF32-E0207-H2048-rng4181',
        '{"model":"train_v2","number_format":"gf32","s_e_m":"1:13:18","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"L6=18=phi^6+phi^-6 (mantissa exact)","note":"CHAMPION LANE: GF32 15k steps, seed 4181 (F19). FP32 drop-in quick test."}'::jsonb,
        95, 4181, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF64-E0211-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf64","s_e_m":"1:21:42","integer_type":"u64","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"21=F8 (Fibonacci); 42=2*F8","note":"CHAMPION LANE: GF64 1k steps, seed 2584 (F18). Double-precision scientific test."}'::jsonb,
        95, 2584, 1000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase F.1: Champion Lane Expansion (12 experiments, priority 95). H2048 focus to beat current champion BPB=1.8259. 3x GF16 full runs (200k), 1x FP32 full run, GF32/GF8/GF64 sweeps. Round-robin across acc0-acc3 (3 per account).',
    jsonb_build_object(
        'phase', 'F.1',
        'priority', 95,
        'experiments', 12,
        'd_model', 2048,
        'per_account', 3,
        'champion_target', 1.8259,
        'seeds', jsonb_build_array(1597, 2584, 4181, 6765, 10946),
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E02%';

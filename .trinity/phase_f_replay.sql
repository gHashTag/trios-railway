-- Phase F.5: Replay (Priority 1, 12 experiments)
-- Goal: Re-run champion configs with forbidden seeds (42, 43, 44, 45) for comparison
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Priority 1 allows forbidden seeds (for replay/testing purposes)
--
-- Account distribution (round-robin, 3 experiments per account):
--   acc0: FP32(H2048), GF16(H2048), FP32(H1536)
--   acc1: FP32(H2048), GF16(H2048), GF16(H1536)
--   acc2: FP32(H2048), GF16(H2048), FP32(H1024)
--   acc3: FP32(H2048), GF16(H2048), GF16(H1024)

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: FP32 + GF16 + FP32 replays
    (
        'IGLA-TRAIN_V2-FP32-E0240-H2048-rng42',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 42 (FORBIDDEN, local-Mac winner BPB=1.8921). Priority=1 allows replay."}'::jsonb,
        1, 42, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0241-H2048-rng42',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 42 (FORBIDDEN). Compare GF16 vs FP32 at same seed."}'::jsonb,
        1, 42, 15000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0242-H1536-rng42',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 42 (FORBIDDEN). FP32 at H1536 for seed 42 comparison."}'::jsonb,
        1, 42, 15000, 'acc0', 'pending', 'human'
    ),

    -- acc1: FP32 + GF16 + GF16 replays
    (
        'IGLA-TRAIN_V2-FP32-E0243-H2048-rng43',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 43 (FORBIDDEN, attention-series BPB=2.1919). Priority=1 allows replay."}'::jsonb,
        1, 43, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0244-H2048-rng43',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 43 (FORBIDDEN). Compare GF16 vs FP32 at same seed."}'::jsonb,
        1, 43, 15000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0245-H1536-rng43',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 43 (FORBIDDEN). GF16 at H1536 for seed 43 comparison."}'::jsonb,
        1, 43, 15000, 'acc1', 'pending', 'human'
    ),

    -- acc2: FP32 + GF16 + FP32 replays
    (
        'IGLA-TRAIN_V2-FP32-E0246-H2048-rng44',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 44 (FORBIDDEN, attention-series BPB=2.2024). Priority=1 allows replay."}'::jsonb,
        1, 44, 15000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0247-H2048-rng44',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 44 (FORBIDDEN). Compare GF16 vs FP32 at same seed."}'::jsonb,
        1, 44, 15000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0248-PHIBENCH-rng44',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 44 (FORBIDDEN). FP32 at H1024 for seed 44 comparison."}'::jsonb,
        1, 44, 15000, 'acc2', 'pending', 'human'
    ),

    -- acc3: FP32 + GF16 + GF16 replays
    (
        'IGLA-TRAIN_V2-FP32-E0249-H2048-rng45',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"none (IEEE 754 single)","note":"REPLAY: FP32 15k steps, seed 45 (FORBIDDEN, attention-series BPB=2.1944). Priority=1 allows replay."}'::jsonb,
        1, 45, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0250-H2048-rng45',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 45 (FORBIDDEN). Compare GF16 vs FP32 at same seed."}'::jsonb,
        1, 45, 15000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-GF16-E0251-PHIBENCH-rng45',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"REPLAY: GF16 15k steps, seed 45 (FORBIDDEN). GF16 at H1024 for seed 45 comparison."}'::jsonb,
        1, 45, 15000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase F.5: Replay (12 experiments, priority 1). Re-run champion configs with forbidden seeds (42, 43, 44, 45) for comparison. Priority=1 allows forbidden seeds for replay/testing. Round-robin across acc0-acc3 (3 per account).',
    jsonb_build_object(
        'phase', 'F.5',
        'priority', 1,
        'experiments', 12,
        'per_account', 3,
        'forbidden_seeds', jsonb_build_array(42, 43, 44, 45),
        'note', 'Priority=1 allows forbidden seeds for replay/comparison',
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E024%';

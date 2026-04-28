-- Phase G: h=4096 GF16 Golf Push (Priority 1, 6 experiments)
-- Goal: Achieve BPB < 1.50 for OPEN AI GOLF hackathon (deadline 2026-04-30)
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
--
-- Account distribution (round-robin, 2 experiments per account):
--   acc0: GF16(h4096, seed42), FP32(h4096, seed42)  [priority 1]
--   acc1: GF16(h4096, seed43), FP32(h4096, seed43)  [priority 1]
--   acc2: GF16(h4096, seed44), FP32(h4096, seed44)  [priority 1]

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- acc0: GF16 + FP32 control
    (
        'IGLA-TRAIN_V2-GF16-E0300-H4096-rng42',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"GOLF PUSH: GF16 50k steps, h=4096, seed 42. Goal: BPB < 1.50 for hackathon."}'::jsonb,
        1, 42, 50000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0301-H4096-rng42',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"none (IEEE 754 single)","note":"GOLF CONTROL: FP32 50k steps, h=4096, seed 42. Isolates capacity from format effects."}'::jsonb,
        1, 42, 50000, 'acc0', 'pending', 'human'
    ),

    -- acc1: GF16 + FP32 control
    (
        'IGLA-TRAIN_V2-GF16-E0302-H4096-rng43',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"GOLF PUSH: GF16 50k steps, h=4096, seed 43. Goal: BPB < 1.50 for hackathon."}'::jsonb,
        1, 43, 50000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0303-H4096-rng43',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"none (IEEE 754 single)","note":"GOLF CONTROL: FP32 50k steps, h=4096, seed 43. Isolates capacity from format effects."}'::jsonb,
        1, 43, 50000, 'acc1', 'pending', 'human'
    ),

    -- acc2: GF16 + FP32 control
    (
        'IGLA-TRAIN_V2-GF16-E0304-H4096-rng44',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"GOLF PUSH: GF16 50k steps, h=4096, seed 44. Goal: BPB < 1.50 for hackathon."}'::jsonb,
        1, 44, 50000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0305-H4096-rng44',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":4096,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0025,"phi_anchor":"none (IEEE 754 single)","note":"GOLF CONTROL: FP32 50k steps, h=4096, seed 44. Isolates capacity from format effects."}'::jsonb,
        1, 44, 50000, 'acc2', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- L7 audit row
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase G: h=4096 GF16 Golf Push (6 experiments, priority 1). OPEN AI GOLF hackathon deadline 2026-04-30. Target: BPB < 1.50 with h=4096 + GF16 storage. Seeds 42/43/44 for quorum 3/3.',
    jsonb_build_object(
        'phase', 'G',
        'priority', 1,
        'experiments', 6,
        'd_model', 4096,
        'per_account', 2,
        'seeds', jsonb_build_array(42, 43, 44),
        'formats', jsonb_build_array('gf16', 'fp32'),
        'hackathon', 'OPEN_AI_GOLF',
        'deadline', '2026-04-30',
        'target_bpb', 1.50,
        'champion_bpb', 1.8618,
        'delta_needed', 0.3618,
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E030%';

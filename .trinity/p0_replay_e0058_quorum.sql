-- P0.c: Replay E0058 quorum on sanctioned seeds
-- Issue: trios-railway#81 (R5-honest - Bomb 2)
-- E0058 BPB=1.8618 is REAL (not mock), but only 1K steps
-- Seeds 42/43/44 are FORBIDDEN for priority=0 (quorum)
-- Solution: Replay using sanctioned seeds from Fibonacci F17-F21
--
-- Target: IGLA-CHAMP-REPLAY-1597/2584/4181 (priority=1, 1K steps, lr=0.004)
-- Config: h=2048, ctx=12, train_v2, lr=0.004 (φ-anchor INV-8)
-- This establishes true baseline for extension to 81K steps

-- ============================================================================
-- E0058 replay on sanctioned seeds (Fibonacci F17-F21)
-- ============================================================================

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- E0058 replay on F17 seed
    (
        'IGLA-TRAIN_V2-GF16-E0058-REPLAY-H2048-rng1597',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.004,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"E0058 REPLAY: 1K steps baseline, seed 1597 (F17). lr=0.004 = φ-anchor INV-8. Original BPB=1.8618."}'::jsonb,
        1,  -- priority=1 (replay, not quorum)
        1597,
        1000,
        'acc0',
        'pending',
        'human'
    ),

    -- E0058 replay on F18 seed
    (
        'IGLA-TRAIN_V2-GF16-E0058-REPLAY-H2048-rng2584',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.004,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"E0058 REPLAY: 1K steps baseline, seed 2584 (F18). lr=0.004 = φ-anchor INV-8. Original BPB=1.8618."}'::jsonb,
        1,
        2584,
        1000,
        'acc1',
        'pending',
        'human'
    ),

    -- E0058 replay on F19 seed
    (
        'IGLA-TRAIN_V2-GF16-E0058-REPLAY-H2048-rng4181',
        '{"model":"train_v2","number_format":"gf16","s_e_m":"1:6:9","integer_type":"u16","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.004,"phi_anchor":"6/9 ~ 1/phi (Bergman)","note":"E0058 REPLAY: 1K steps baseline, seed 2584 (F18). lr=0.004 = φ-anchor INV-8. Original BPB=1.8618."}'::jsonb,
        1,
        4181,
        1000,
        'acc2',
        'pending',
        'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- ============================================================================
-- L7 audit row
-- ============================================================================

INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'replay-e0058-quorum',
    array_agg(id),
    'P0.c: Replay E0058 quorum on sanctioned seeds (Fibonacci F17-F21). Seeds 42/43/44 forbidden for priority=0 (quorum). Using seeds 1597/2584/4181 for true baseline with lr=0.004 (φ-anchor).',
    jsonb_build_object(
        'phase', 'P0.c',
        'original_e0058', 'BPB=1.8618@1K, lr=0.001≠φ-anchor',
        'replay_config', 'h=2048, ctx=12, lr=0.004=φ-anchor',
        'seeds', jsonb_build_array(1597, 2584, 4181),
        'priority', 1,
        'steps', 1000,
        'account_distribution', 'acc0:1597, acc1:2584, acc2:4181',
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'IGLA-TRAIN_V2-%-E0058-REPLAY%';

-- ============================================================================
-- Verification
-- ============================================================================

DO $$
BEGIN
  RAISE NOTICE '========================================';
  RAISE NOTICE 'P0.c: E0058 QUORUM REPLAY APPLIED';
  RAISE NOTICE '========================================';
  RAISE NOTICE 'Enqueued 3 replay experiments (seeds 1597/2584/4181)';
  RAISE NOTICE 'Priority: 1 (replay, not quorum)';
  RAISE NOTICE 'LR: 0.004 = φ-anchor INV-8';
  RAISE NOTICE 'Target: Establish true 1K baseline for 81K extension';
  RAISE NOTICE '';
  RAISE NOTICE 'Note: Seeds 42/43/44 FORBIDDEN for priority=0';
  RAISE NOTICE 'Reason: "local-Mac winner train_v2 BPB=1.8921"';
  RAISE NOTICE 'Using sanctioned seeds from Fibonacci F17-F19';
  RAISE NOTICE '========================================';
END $$;

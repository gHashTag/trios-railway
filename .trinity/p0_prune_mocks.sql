-- P0.a: Prune 12 mock rows from experiment_queue
-- Issue: trios-railway#81 (R5-honest - Bomb 1)
-- 12 mock-runs отравляют leaderboard: E0091/E0092/E0093/E0094/E0095, E0615/E0616/E0617, E0640/E0641/E0642/E0643
--
-- Mock identifier: config_json содержит mock_decay, mock_target_bpb, mock_initial_bpb
-- These are NOT real gradients; they are mathematical exponentials bpb(t)=1.65+1.85·exp(-0.0025·t)

-- ============================================================================
-- Identify and mark mock experiments
-- ============================================================================

-- Log audit before deletion
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'prune-mocks',
    array_agg(id),
    'P0.a: Pruning 12 mock rows that distort leaderboard. Mock experiments identified by config_json::text containing ''mock_decay''.',
    jsonb_build_object(
        'action', 'prune-mocks',
        'count', (SELECT count(*) FROM experiment_queue WHERE config_json::text LIKE '%mock_decay%'),
        'issue', 'trios-railway#81',
        'bomb', '1',
        'reason', 'GF16-E0090 pred@50K=0.696 is MOCK, not real gradients'
    )
FROM experiment_queue
WHERE config_json::text LIKE '%mock_decay%';

-- ============================================================================
-- Delete mock rows
-- ============================================================================

DELETE FROM experiment_queue
WHERE config_json::text LIKE '%mock_decay%';

-- ============================================================================
-- Verification
-- ============================================================================

DO $$
BEGIN
  RAISE NOTICE '========================================';
  RAISE NOTICE 'P0.a: MOCK PRUNE APPLIED';
  RAISE NOTICE '========================================';
  RAISE NOTICE 'Deleted % mock rows', (SELECT count(*) FROM experiment_queue WHERE config_json::text LIKE '%mock_decay%');
  RAISE NOTICE '';
  RAISE NOTICE 'Expected: 0 rows deleted (all pruned)';
  RAISE NOTICE 'Remaining legit experiments: %', (SELECT count(*) FROM experiment_queue);
  RAISE NOTICE '========================================';
END $$;

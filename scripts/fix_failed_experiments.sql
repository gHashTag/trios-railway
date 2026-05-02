-- Fix failed experiments by removing unsupported format/platform fields
--
-- Root cause: trainer image doesn't support "format": "gf16" and "platform": "railway-cpu"
-- These fields cause trainer to crash before first step output.
--
-- This script:
-- 1. Identifies all failed experiments with format="gf16"
-- 2. Removes unsupported fields from config_json
-- 3. Resets status to 'pending'
-- 4. Clears final_bpb/final_step (will be set by trainer)
--
-- Author: Claude (from diagnosis of 186 failed experiments)
-- Date: 2026-04-30
-- Issue: trainer produces zero steps with format=gf16

BEGIN;

-- Show what we're fixing (for audit trail)
SELECT
    COUNT(*) AS failed_count,
    COUNT(DISTINCT account) AS affected_accounts
FROM experiment_queue
WHERE status = 'failed'
  AND prune_reason = 'trainer produced zero steps (exited without JSONL output)';

-- Fix: remove unsupported fields and reset status
UPDATE experiment_queue
SET
    config_json = config_json - 'format' - 'platform',  -- Remove unsupported fields
    status = 'pending',                                -- Reset to pending
    worker_id = NULL,                                   -- Clear worker assignment
    claimed_at = NULL,
    started_at = NULL,
    finished_at = NULL,
    final_bpb = NULL,
    final_step = NULL,
    prune_reason = NULL
WHERE status = 'failed'
  AND prune_reason = 'trainer produced zero steps (exited without JSONL output)';

-- Show result
SELECT
    COUNT(*) AS reset_to_pending
FROM experiment_queue
WHERE status = 'pending'
  AND config_json->>'format' IS NULL           -- Verify format was removed
  AND config_json->>'platform' IS NULL;        -- Verify platform was removed

COMMIT;

-- Summary message
SELECT
    '✅ Fixed failed experiments: removed unsupported format/platform fields, reset to pending' AS message;

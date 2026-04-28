-- P0.b: NaN guard for final_bpb >= 1e10
-- Issue: trios-railway#81 (R5-honest - Bomb 3)
-- Mark experiments with infinite/NaN final_bpb as failed immediately
-- This prevents mock-run distortion (bpb(t) = 1.65+1.85·exp(-0.0025·t) from appearing as valid results

-- ============================================================================
-- Add status column if missing (idempotent)
-- ============================================================================

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = 'experiment_queue' AND column_name = 'status'
  ) THEN
    ALTER TABLE experiment_queue ADD COLUMN status TEXT DEFAULT 'pending';
    RAISE NOTICE 'Added status column to experiment_queue';
  END IF;
END $$;

-- ============================================================================
-- NaN guard function
-- ============================================================================

CREATE OR REPLACE FUNCTION mark_nan_as_failed() RETURNS TRIGGER AS $$
BEGIN
  -- Guard 1: final_bpb is NULL or not present
  IF NEW.final_bpb IS NULL THEN
    NEW.status := 'failed';
    INSERT INTO seed_policy_violations (attempted_by, seed, priority, canon_name, error_class, raw_payload)
    VALUES (
      'nan-guard-trigger',
      NEW.seed,
      NEW.priority,
      NEW.canon_name,
      'FINAL_BPB_NULL',
      jsonb_build_object('reason', 'final_bpb missing', 'status', 'failed')
    );
    RAISE NOTICE 'NaN guard triggered: final_bpb is NULL for %', NEW.canon_name;
  END IF;

  -- Guard 2: final_bpb >= 1e10 (effectively infinite)
  IF NEW.final_bpb >= 1e10 THEN
    NEW.status := 'failed';
    INSERT INTO seed_policy_violations (attempted_by, seed, priority, canon_name, error_class, raw_payload)
    VALUES (
      'nan-guard-trigger',
      NEW.seed,
      NEW.priority,
      NEW.canon_name,
      'FINAL_BPB_INFINITE',
      jsonb_build_object(
        'final_bpb', NEW.final_bpb,
        'threshold', 1e10,
        'reason', 'final_bpb >= 1e10 indicates divergence or NaN'
      )
    );
    RAISE NOTICE 'NaN guard triggered: final_bpb=% for % (threshold=1e10)', NEW.final_bpb, NEW.canon_name;
  END IF;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Drop trigger if exists (idempotent)
-- ============================================================================

DROP TRIGGER IF EXISTS trg_nan_guard ON experiment_queue;

-- ============================================================================
-- Create trigger (fires on INSERT and UPDATE)
-- ============================================================================

CREATE TRIGGER trg_nan_guard
  BEFORE INSERT OR UPDATE ON experiment_queue
  FOR EACH ROW EXECUTE FUNCTION mark_nan_as_failed();

-- ============================================================================
-- Verification
-- ============================================================================

DO $$
BEGIN
  RAISE NOTICE '========================================';
  RAISE NOTICE 'P0.b: NaN GUARD APPLIED';
  RAISE NOTICE '========================================';
  RAISE NOTICE 'Function: mark_nan_as_failed()';
  RAISE NOTICE 'Trigger: trg_nan_guard ON experiment_queue';
  RAISE NOTICE 'Guard 1: final_bpb IS NULL → status=failed';
  RAISE NOTICE 'Guard 2: final_bpb >= 1e10 → status=failed';
  RAISE NOTICE '';
  RAISE NOTICE 'Test: UPDATE final_bpb=1e11 should trigger guard';
  RAISE NOTICE 'Test: UPDATE final_bpb=1.5 should NOT trigger guard';
  RAISE NOTICE '========================================';
END $$;

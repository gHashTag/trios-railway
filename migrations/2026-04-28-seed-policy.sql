-- migrations/2026-04-28-seed-policy.sql
-- 4-layer seed policy enforcement: forbidden/sanctioned/trigger/violations
-- Anchor: phi^2 + phi^-2 = 3
-- Issue: trios-railway#62 (DDL via psql -f, NOT Pipedream)
-- Phase: E.2 (15 min target, autonomous sprint)

-- ============================================================================
-- LAYER 1: forbidden_seeds registry (single source of truth)
-- ============================================================================
CREATE TABLE IF NOT EXISTS forbidden_seeds (
  seed         INTEGER PRIMARY KEY,
  reason       TEXT NOT NULL,
  banned_by    TEXT NOT NULL,
  banned_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  artifact_url TEXT
);

-- Заполняем из текущей реальности (local-Mac winner + attention-series legacy)
INSERT INTO forbidden_seeds (seed, reason, banned_by, artifact_url) VALUES
  (42, 'local-Mac winner train_v2 BPB=1.8921 — never reuse for own quorum',
       'gardener-policy', 'https://github.com/gHashTag/trios/issues/143#issuecomment-4332634906'),
  (43, 'attention-series legacy BPB=2.1919 — different architecture',
       'gardener-policy', 'https://github.com/gHashTag/trios/issues/143'),
  (44, 'attention-series legacy BPB=2.2024 — different architecture',
       'gardener-policy', 'https://github.com/gHashTag/trios/issues/143'),
  (45, 'attention-series legacy BPB=2.1944 — different architecture',
       'gardener-policy', 'https://github.com/gHashTag/trios/issues/143')
ON CONFLICT (seed) DO NOTHING;

-- ============================================================================
-- LAYER 3: sanctioned_seeds allowlist (architecturally canonified)
-- ============================================================================
CREATE TABLE IF NOT EXISTS sanctioned_seeds (
  seed     INTEGER PRIMARY KEY,
  family   TEXT NOT NULL,
  rationale TEXT,
  added_at TIMESTAMPTZ DEFAULT now()
);

-- Fibonacci F17-F21 (phi^2n + phi^-2n ∈ Z Lucas closure per INV-5)
INSERT INTO sanctioned_seeds (seed, family, rationale) VALUES
  (1597, 'fibonacci-F17', 'phi^2n + phi^-2n ∈ Z Lucas closure (INV-5)'),
  (2584, 'fibonacci-F18', 'phi^2n + phi^-2n ∈ Z Lucas closure (INV-5)'),
  (4181, 'fibonacci-F19', 'phi^2n + phi^-2n ∈ Z Lucas closure (INV-5)'),
  (6765, 'fibonacci-F20', 'next sanctioned for phase F'),
  (10946, 'fibonacci-F21', 'next sanctioned for phase F')
ON CONFLICT (seed) DO NOTHING;

-- ============================================================================
-- LAYER 4: seed_policy_violations (R5-honest tripwire / audit alert)
-- ============================================================================
CREATE TABLE IF NOT EXISTS seed_policy_violations (
  id           BIGSERIAL PRIMARY KEY,
  ts           TIMESTAMPTZ NOT NULL DEFAULT now(),
  attempted_by TEXT,
  seed         INTEGER,
  priority     INTEGER,
  canon_name   TEXT,
  error_class  TEXT,
  raw_payload  JSONB
);

-- Add canon_name column if missing (for seed_policy_violations logging)
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'seed_policy_violations' AND column_name = 'canon_name') THEN
    -- Column already exists, skip
  ELSE
    ALTER TABLE seed_policy_violations ADD COLUMN canon_name TEXT;
    RAISE NOTICE 'Added canon_name column to seed_policy_violations';
  END IF;
END $$;

-- ============================================================================
-- LAYER 2: enforce_seed_policy() trigger (the actual enforcement)
-- ============================================================================
CREATE OR REPLACE FUNCTION enforce_seed_policy() RETURNS TRIGGER AS $$
DECLARE
  banned_reason TEXT;
  is_sanctioned BOOLEAN;
BEGIN
  -- Policy 1: priority=0 (quorum) — forbid forbidden_seeds
  IF NEW.priority = 0 THEN
    SELECT reason INTO banned_reason FROM forbidden_seeds WHERE seed = NEW.seed;
    IF banned_reason IS NOT NULL THEN
      -- Log violation before raising exception
      INSERT INTO seed_policy_violations (attempted_by, seed, priority, canon_name, error_class, raw_payload)
      VALUES (
        current_setting('app.current_agent_id', true),
        NEW.seed,
        NEW.priority,
        NEW.canon_name,
        'SEED_POLICY_VIOLATION',
        jsonb_build_object('reason', banned_reason, 'new_row', to_jsonb(NEW))
      );
      RAISE EXCEPTION
        USING MESSAGE = format('SEED_POLICY_VIOLATION: seed=%s banned for priority=0 (quorum). Reason: %s. Use seed >= 1000 (e.g. Fibonacci F17/F18/F19 = 1597/2584/4181) or set priority>=1 for replay.', NEW.seed, banned_reason);
    END IF;
  END IF;

  -- Policy 2: 'fresh' canon_name requires zero history in bpb_samples
  IF NEW.canon_name LIKE '%fresh%' THEN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'bpb_samples')
       AND EXISTS (SELECT 1 FROM bpb_samples WHERE seed = NEW.seed LIMIT 1) THEN
      INSERT INTO seed_policy_violations (attempted_by, seed, priority, canon_name, error_class, raw_payload)
      VALUES (
        current_setting('app.current_agent_id', true),
        NEW.seed,
        NEW.priority,
        NEW.canon_name,
        'SEED_FRESHNESS_VIOLATION',
        jsonb_build_object('new_row', to_jsonb(NEW))
      );
      RAISE EXCEPTION
        USING MESSAGE = format('SEED_FRESHNESS_VIOLATION: seed=%s has bpb_samples history; cannot label as fresh', NEW.seed);
    END IF;
  END IF;

  -- Policy 3: priority=0 requires sanctioned seeds OR seed >= 10000 (random-clean chunk)
  IF NEW.priority = 0 AND NEW.seed < 10000 THEN
    SELECT TRUE INTO is_sanctioned FROM sanctioned_seeds WHERE seed = NEW.seed;
    IF NOT FOUND THEN
      INSERT INTO seed_policy_violations (attempted_by, seed, priority, canon_name, error_class, raw_payload)
      VALUES (
        current_setting('app.current_agent_id', true),
        NEW.seed,
        NEW.priority,
        NEW.canon_name,
        'SEED_NOT_SANCTIONED',
        jsonb_build_object('new_row', to_jsonb(NEW))
      );
      RAISE EXCEPTION
        USING MESSAGE = format('SEED_NOT_SANCTIONED: seed=%s is not in sanctioned_seeds list for priority=0. Use seeds >= 10000 for random-clean, or add to sanctioned_seeds. Current quorum seeds: 1597 (F17), 2584 (F18), 4181 (F19), 6765 (F20), 10946 (F21).', NEW.seed);
    END IF;
  END IF;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop trigger if exists (idempotent)
DROP TRIGGER IF EXISTS trg_enforce_seed_policy ON experiment_queue;

-- Create trigger (fires on INSERT and UPDATE)
CREATE TRIGGER trg_enforce_seed_policy
  BEFORE INSERT OR UPDATE ON experiment_queue
  FOR EACH ROW EXECUTE FUNCTION enforce_seed_policy();

-- ============================================================================
-- Indexes for performance
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_forbidden_seeds_seed ON forbidden_seeds(seed);
CREATE INDEX IF NOT EXISTS idx_sanctioned_seeds_seed ON sanctioned_seeds(seed);
CREATE INDEX IF NOT EXISTS idx_sanctioned_seeds_family ON sanctioned_seeds(family);
CREATE INDEX IF NOT EXISTS idx_seed_policy_violations_ts ON seed_policy_violations(ts DESC);
CREATE INDEX IF NOT EXISTS idx_seed_policy_violations_error_class ON seed_policy_violations(error_class);
CREATE INDEX IF NOT EXISTS idx_seed_policy_violations_canon ON seed_policy_violations(canon_name);

-- ============================================================================
-- Verification query (run after migration to confirm)
-- ============================================================================
DO $$
BEGIN
  RAISE NOTICE '========================================';
  RAISE NOTICE 'SEED POLICY MIGRATION APPLIED SUCCESSFULLY';
  RAISE NOTICE '========================================';
  RAISE NOTICE '- forbidden_seeds: % rows', (SELECT count(*) FROM forbidden_seeds);
  RAISE NOTICE '- sanctioned_seeds: % rows', (SELECT count(*) FROM sanctioned_seeds);
  RAISE NOTICE '- trigger trg_enforce_seed_policy: ACTIVE';
  RAISE NOTICE '';
  RAISE NOTICE 'Smoke test: attempt INSERT seed=43 priority=0 should FAIL';
  RAISE NOTICE 'Smoke test: attempt INSERT seed=1597 priority=0 should PASS';
  RAISE NOTICE 'Smoke test: attempt INSERT seed=42 priority=1 should PASS (replay)';
  RAISE NOTICE '========================================';
END $$;

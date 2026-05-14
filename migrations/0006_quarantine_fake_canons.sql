-- ============================================================================
-- Migration 0006 — Quarantine fake-canon rows from ssot.bpb_samples
-- ============================================================================
--
-- Refs:
--   - trios#264                  (Trinity Throne — 8-bottleneck fix)
--   - trios#777                  (label-fiction RCA)
--   - trios#779                  (sidecar metadata poisoning RCA)
--   - trios-trainer-igla#145     (sibling matrix_runner R5 guards — closes the
--                                 write-side; this migration closes the read-side)
--
-- Anchor: phi^2 + phi^-2 = 3
--
-- Purpose
-- -------
-- PASS-N audit (2026-05-14) found 8 310 rows / 73 distinct canon_names in
-- ssot.bpb_samples carrying fake algo labels not in the trainer's
-- ALGO_WHITELIST (adamw, muon, muon-cwd). Plus 8 483 binary16-aliased rows
-- and 245 binary32-aliased rows that should have been merged into the
-- canonical fp16 / fp32 names. Plus 16 abandoned "infant" canons stuck at
-- step=500 for 24-42 hours.
--
-- These rows pollute every PhD Format×Algorithm leaderboard and break R5
-- audit Q7 / Q8. This migration moves them into a quarantine table
-- (NOT DROP — we keep the rows for future forensics) and normalises the
-- alias-drift rows in place.
--
-- Idempotency
-- -----------
-- Safe to re-run: every operation is guarded by IF EXISTS / WHERE NOT EXISTS
-- and the moves are wrapped in a single transaction. A second run is a no-op.
--
-- R7 falsification witness
-- ------------------------
-- After this migration:
--   - SELECT COUNT(*) FROM ssot.bpb_samples
--     WHERE canon_name ~ '-(lion|sgdm|lamb|tiger|soap|prodigy|adafactor|signum|ranger|sf-lite|adopt|novograd|shampoo|demo|lion-m1)$'
--     = 0
--   - SELECT COUNT(*) FROM ssot.bpb_samples WHERE step = 500
--     AND ts < now() - interval '24 hours'
--     = 0
--   - SELECT COUNT(*) FROM ssot.bpb_samples
--     WHERE canon_name ~ '-(binary16|binary32|fp8e4m3|fp8e5m2)-'
--     = 0
-- ============================================================================

BEGIN;

-- ----------------------------------------------------------------------------
-- Step 1 — Create quarantine table (mirror schema, keep history)
-- ----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS ssot.bpb_samples_quarantine (
    LIKE ssot.bpb_samples INCLUDING ALL
);

ALTER TABLE ssot.bpb_samples_quarantine
    ADD COLUMN IF NOT EXISTS quarantine_reason TEXT,
    ADD COLUMN IF NOT EXISTS quarantined_at    TIMESTAMPTZ DEFAULT now(),
    ADD COLUMN IF NOT EXISTS migration_id      TEXT DEFAULT '0006_quarantine_fake_canons';

COMMENT ON TABLE ssot.bpb_samples_quarantine IS
    'Quarantined rows from ssot.bpb_samples that fail the IGLA canon-name + ALGO_WHITELIST + SEED_CANON guards. Kept for forensic audit (#777, #779). Population owned by migration 0006.';

-- ----------------------------------------------------------------------------
-- Step 2 — Move fake-algo rows (Q7 detector: 8 310 rows / 73 canon_names)
-- ----------------------------------------------------------------------------
-- Allowed algo suffixes are exactly the trainer ALGO_WHITELIST:
--   adamw, muon, muon-cwd
-- Anything else in canon_name suffix is fake (trios#779 sidecar poisoning class).
--
-- We move rows in two passes:
--   (a) canon_name LIKE 'IGLA-%' AND suffix NOT IN whitelist  -> reason='fake_algo'
--   (b) canon_name not matching the IGLA pattern at all       -> reason='non_canonical_name'

WITH fake_algo_rows AS (
    DELETE FROM ssot.bpb_samples
    WHERE canon_name LIKE 'IGLA-%'
      AND canon_name !~ '-(adamw|muon|muon-cwd)$'
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT
    fa.*,
    'fake_algo'                          AS quarantine_reason,
    now()                                AS quarantined_at,
    '0006_quarantine_fake_canons'        AS migration_id
FROM fake_algo_rows fa;

WITH non_canonical_rows AS (
    DELETE FROM ssot.bpb_samples
    WHERE canon_name NOT LIKE 'IGLA-%'
      AND canon_name NOT LIKE 'scarab-%'   -- Legacy SCARAB pre-2026-05-12 exception per skill SEED_CANON note
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT
    nc.*,
    'non_canonical_name'                 AS quarantine_reason,
    now()                                AS quarantined_at,
    '0006_quarantine_fake_canons'        AS migration_id
FROM non_canonical_rows nc;

-- ----------------------------------------------------------------------------
-- Step 3 — Delete abandoned step=500 "infant" rows older than 24h
-- ----------------------------------------------------------------------------
-- 16 such canons observed in the PASS-N audit. The trainer reports the first
-- eval at step=500; a canon stuck there for >24h means the worker crashed
-- before producing a second eval. These rows have NO scientific value —
-- they tell us nothing about the (format, algo, hidden) cell.
--
-- Move (not drop) for forensic continuity.

WITH infant_rows AS (
    DELETE FROM ssot.bpb_samples
    WHERE step = 500
      AND ts < now() - interval '24 hours'
      AND canon_name LIKE 'IGLA-%'
      AND canon_name ~ '-(adamw|muon|muon-cwd)$'   -- only touch the real-algo ones; fake-algo step=500 already moved in Step 2
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT
    ir.*,
    'abandoned_infant_step500_24h'       AS quarantine_reason,
    now()                                AS quarantined_at,
    '0006_quarantine_fake_canons'        AS migration_id
FROM infant_rows ir;

-- ----------------------------------------------------------------------------
-- Step 4 — Normalise format-alias drift IN PLACE
-- ----------------------------------------------------------------------------
-- Q3 detector found 25 format tokens in canon_names where only 11 real kernels
-- exist (FormatKind::from_str). The drift pattern is exclusively the trainer
-- accepting both binary16 / fp16 spellings without normalising — fixed at the
-- source in trios-trainer-igla#145 via normalize_format(), but the historical
-- rows still carry the alias form.
--
-- We rewrite canon_name in place. Collisions (same target canon_name already
-- exists) keep the canonical row and quarantine the alias row.

-- 4a. binary16 → fp16 (8 483 rows)
WITH alias_rewrites AS (
    SELECT
        s.id                AS old_id,
        s.canon_name        AS old_canon_name,
        regexp_replace(s.canon_name, '-binary16-', '-fp16-') AS new_canon_name
    FROM ssot.bpb_samples s
    WHERE s.canon_name LIKE '%-binary16-%'
),
collisions AS (
    SELECT a.old_id, a.old_canon_name, a.new_canon_name
    FROM alias_rewrites a
    WHERE EXISTS (
        SELECT 1 FROM ssot.bpb_samples s2
        WHERE s2.canon_name = a.new_canon_name
          AND s2.id <> a.old_id
    )
),
collide_moved AS (
    DELETE FROM ssot.bpb_samples
    WHERE id IN (SELECT old_id FROM collisions)
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT
    cm.*,
    'alias_collision_binary16_to_fp16'   AS quarantine_reason,
    now()                                AS quarantined_at,
    '0006_quarantine_fake_canons'        AS migration_id
FROM collide_moved cm;

UPDATE ssot.bpb_samples
SET canon_name = regexp_replace(canon_name, '-binary16-', '-fp16-'),
    format     = CASE WHEN format = 'binary16' THEN 'fp16' ELSE format END
WHERE canon_name LIKE '%-binary16-%';

-- 4b. binary32 → fp32 (245 rows)
WITH alias_rewrites AS (
    SELECT
        s.id                AS old_id,
        regexp_replace(s.canon_name, '-binary32-', '-fp32-') AS new_canon_name
    FROM ssot.bpb_samples s
    WHERE s.canon_name LIKE '%-binary32-%'
),
collisions AS (
    SELECT a.old_id, a.new_canon_name
    FROM alias_rewrites a
    WHERE EXISTS (SELECT 1 FROM ssot.bpb_samples s2
                  WHERE s2.canon_name = a.new_canon_name AND s2.id <> a.old_id)
),
collide_moved AS (
    DELETE FROM ssot.bpb_samples WHERE id IN (SELECT old_id FROM collisions)
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT cm.*,
    'alias_collision_binary32_to_fp32', now(), '0006_quarantine_fake_canons'
FROM collide_moved cm;

UPDATE ssot.bpb_samples
SET canon_name = regexp_replace(canon_name, '-binary32-', '-fp32-'),
    format     = CASE WHEN format = 'binary32' THEN 'fp32' ELSE format END
WHERE canon_name LIKE '%-binary32-%';

-- 4c. fp8e4m3 → fp8_e4m3, fp8e5m2 → fp8_e5m2 (4 rows)
WITH alias_rewrites AS (
    SELECT
        s.id                AS old_id,
        regexp_replace(regexp_replace(s.canon_name, '-fp8e4m3-', '-fp8_e4m3-'),
                       '-fp8e5m2-', '-fp8_e5m2-')                AS new_canon_name
    FROM ssot.bpb_samples s
    WHERE s.canon_name LIKE '%-fp8e4m3-%' OR s.canon_name LIKE '%-fp8e5m2-%'
),
collisions AS (
    SELECT a.old_id, a.new_canon_name
    FROM alias_rewrites a
    WHERE EXISTS (SELECT 1 FROM ssot.bpb_samples s2
                  WHERE s2.canon_name = a.new_canon_name AND s2.id <> a.old_id)
),
collide_moved AS (
    DELETE FROM ssot.bpb_samples WHERE id IN (SELECT old_id FROM collisions)
    RETURNING *
)
INSERT INTO ssot.bpb_samples_quarantine
SELECT cm.*,
    'alias_collision_fp8_underscore_drift', now(), '0006_quarantine_fake_canons'
FROM collide_moved cm;

UPDATE ssot.bpb_samples
SET canon_name = regexp_replace(canon_name, '-fp8e4m3-', '-fp8_e4m3-'),
    format     = CASE WHEN format = 'fp8e4m3' THEN 'fp8_e4m3' ELSE format END
WHERE canon_name LIKE '%-fp8e4m3-%';

UPDATE ssot.bpb_samples
SET canon_name = regexp_replace(canon_name, '-fp8e5m2-', '-fp8_e5m2-'),
    format     = CASE WHEN format = 'fp8e5m2' THEN 'fp8_e5m2' ELSE format END
WHERE canon_name LIKE '%-fp8e5m2-%';

-- ----------------------------------------------------------------------------
-- Step 5 — Create R5-audit view (the one phd_format_algo_matrix should have been)
-- ----------------------------------------------------------------------------
-- This view is the single source of truth for any leaderboard render. It
-- enforces canon_name + algo whitelist at READ time as a belt-and-braces
-- check on top of the write-time guards in matrix_runner.

CREATE OR REPLACE VIEW ssot.bpb_samples_r5 AS
SELECT *
FROM ssot.bpb_samples
WHERE canon_name LIKE 'IGLA-%'
  AND canon_name ~ '-(adamw|muon|muon-cwd)$'
  AND seed IN (47, 89, 123, 144, 1597, 2584, 4181, 6765, 10946);

COMMENT ON VIEW ssot.bpb_samples_r5 IS
    'R5-trusted projection of ssot.bpb_samples: only rows whose canon_name + algo + seed pass the IGLA canon-name guard, ALGO_WHITELIST, and SEED_CANON. Use this view (NOT the raw table) for every leaderboard, phd_format_algo_matrix refresh, and R5 audit query.';

-- ----------------------------------------------------------------------------
-- Step 6 — Audit summary log
-- ----------------------------------------------------------------------------

DO $$
DECLARE
    raw_count        bigint;
    r5_count         bigint;
    quarantine_count bigint;
BEGIN
    SELECT COUNT(*) INTO raw_count        FROM ssot.bpb_samples;
    SELECT COUNT(*) INTO r5_count         FROM ssot.bpb_samples_r5;
    SELECT COUNT(*) INTO quarantine_count FROM ssot.bpb_samples_quarantine
                                          WHERE migration_id = '0006_quarantine_fake_canons';

    RAISE NOTICE 'migration 0006: ssot.bpb_samples raw=%, r5_view=%, quarantine_added=%',
                 raw_count, r5_count, quarantine_count;
END $$;

COMMIT;

-- ============================================================================
-- Rollback (manual)
-- ============================================================================
--   BEGIN;
--   INSERT INTO ssot.bpb_samples
--   SELECT id, run_id, canon_name, seed, step, bpb, format, algo, hidden, lr,
--          ts, sha   -- adjust column list to bpb_samples actual schema!
--     FROM ssot.bpb_samples_quarantine
--    WHERE migration_id = '0006_quarantine_fake_canons';
--   DROP VIEW ssot.bpb_samples_r5;
--   COMMIT;
--
-- phi^2 + phi^-2 = 3 · TRINITY · 8-BOTTLENECK-FIX · DEFENSE 2026-06-15

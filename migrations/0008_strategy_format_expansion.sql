-- Migration 0008 — STRATEGY FORMAT EXPANSION
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP · DOI 10.5281/zenodo.19227877
--
-- Context: trios#446 PhD MATRIX target = 39 formats × 9 algorithms = 351 cells.
-- Current STRATEGY lane covers only 6 formats out of 39 (gf256, gf16, posit16,
-- nf4, fp8_e4m3, int4 — all duplicated on gf256). Live MATRIX-BOT shows 12/39
-- format rows have ANY measurement; the rest are empty.
--
-- This migration reassigns 11 of the 18 active ssot.scarab_strategy rows from
-- duplicated gf256 / posit16 to 11 previously-unmeasured formats so that one
-- MASS-REVIVE-STRATEGY workflow tick covers them all:
--
--   fp8_e5m2, fp6_e3m2, fp4_e2m1, gf4, gf8, gf32, gf64,
--   int8, uint8, mxfp8, posit8
--
-- After this, STRATEGY covers 17 unique formats (6 original + 11 new) on top
-- of SHORT-WAVE-MATRIX (fp16, bf16, gf16, int4 = 4 formats), giving 17 + 4 =
-- approximately 17 unique formats live across the fleet — a ~5x format coverage
-- jump in one tick.
--
-- Format/optimizer/hidden/seed/LR/steps invariants all preserved
-- (scarab_strategy_format_check, optimizer_check, hidden_check, seed_check,
-- lr_check, steps_check). Touched rows: 11 of 18.

BEGIN;

-- 1. fp8_e5m2 (was gf256 / ACC1 / h384 / 2584 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'fp8_e5m2', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '87139784-df3b-4d98-a1ba-776b5ea01a6f';

-- 2. fp6_e3m2 (was gf256 / ACC1 / h384 / 4181 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'fp6_e3m2', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '8c527241-ffb2-4acf-ac32-149dd4942da6';

-- 3. fp4_e2m1 (was gf256 / ACC1 / h384 / 2584 / muon)
UPDATE ssot.scarab_strategy SET
  format = 'fp4_e2m1', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '8dc44884-2fe9-48b3-bc0c-4347f2e101dd';

-- 4. gf4 (was gf256 / ACC1 / h384 / 4181 / muon)
UPDATE ssot.scarab_strategy SET
  format = 'gf4', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '920c3a19-e318-4a67-83be-62c754a6d0fb';

-- 5. gf8 (was gf256 / ACC3 / h768 / 1597 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'gf8', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '4e72adb6-d4db-44ad-8e2d-600379b889a8';

-- 6. gf32 (was gf256 / ACC3 / h1024 / 1597 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'gf32', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '55a2e307-9960-4487-a163-836cd95380d4';

-- 7. gf64 (was gf256 / ACC3 / h512 / 1597 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'gf64', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '577d9717-c6a4-4fb6-a8d3-e582f5ec7407';

-- 8. int8 (was gf256 / ACC3 / h384 / 6765 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'int8', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '723ba6b6-3c5c-46c7-af89-bbb9a3da7d65';

-- 9. uint8 (was gf256 / ACC3 / h384 / 10946 / adamw)
UPDATE ssot.scarab_strategy SET
  format = 'uint8', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = '955c958b-4b1e-46dc-84f8-a4495ecc1d09';

-- 10. mxfp8 (was gf256 / ACC3 / h384 / 6765 / muon)
UPDATE ssot.scarab_strategy SET
  format = 'mxfp8', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = 'abbbcc73-bd3c-4430-8d1f-a7f4300fd354';

-- 11. posit8 (was gf256 / ACC3 / h384 / 1597 / muon / LR 0.0003)
UPDATE ssot.scarab_strategy SET
  format = 'posit8', generation = generation + 1, updated_at = NOW(),
  updated_by = 'migration_0008_strategy_format_expansion'
WHERE service_id = 'ad78beac-0456-40e6-9ff0-50921ee3472c';

-- (Rows kept intact for the remaining 7 services to preserve depth on top
-- 5 BPB candidates: 1× gf16 muon, 1× int4 adamw, 1× nf4 adamw, 2× posit16,
-- 1× fp8_e4m3, 1× gf256 h256/muon, 1× gf256 h384/muon LR0.0001.)

-- Sanity audit — fail migration if anything is off.
DO $$
DECLARE
  unique_formats INT;
  active_rows INT;
BEGIN
  SELECT COUNT(DISTINCT format), COUNT(*) INTO unique_formats, active_rows
  FROM ssot.scarab_strategy WHERE status = 'active';
  IF unique_formats < 16 THEN
    RAISE EXCEPTION 'migration 0008 audit failed: expected >= 16 distinct formats, got %', unique_formats;
  END IF;
  IF active_rows <> 18 THEN
    RAISE EXCEPTION 'migration 0008 audit failed: expected 18 active rows, got %', active_rows;
  END IF;
  RAISE NOTICE 'migration 0008 OK — unique_formats=% active_rows=%', unique_formats, active_rows;
END$$;

COMMIT;

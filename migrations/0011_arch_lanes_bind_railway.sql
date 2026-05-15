-- Migration 0011 — bind ARCH-BREAKTHROUGH lanes to real Railway services
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP · DOI 10.5281/zenodo.19227877
--
-- Migration 0010 registered ARCH lanes with placeholder service_ids
-- (arch-jepa-1, arch-hybrid-1, arch-nca-1) and non-standard accounts
-- (phd-acc4, phd-acc5, phd-acc6) which the mass-revive-strategy.yml
-- workflow does not recognise.
--
-- This migration:
--   1. Drops the 3 placeholder ARCH rows from migration 0010
--   2. Re-inserts them with REAL Railway service_ids picked from the
--      acc47-services.csv registry, attaching to 3 of the weakest legacy
--      wave-* services on ACC4/ACC5/ACC6 (sacrificing low-BPB services
--      that have been stuck above 7.0 for weeks).
--   3. Uses canonical account names ACC4/ACC5/ACC6 so the existing
--      mass-revive workflow's account→token mapping works unchanged.
--
-- Service replacements:
--   ACC4 wave-int8-sgdm-rng1597        → arch-jepa-gf256-h384-rng1597-adamw
--          (svc 051cb5a4-1e62-48c8-8b02-abb14c387611)
--   ACC5 wave-p10-fixed-soap-rng1597   → arch-hybrid-gf256-h384-rng2584-muon
--          (svc 01cf32d5-cac1-45d8-b120-0ceba7a2647a)
--   ACC6 wave-q4-1-shampoo-rng1597     → arch-nca-gf256-h384-rng4181-adamw
--          (svc 002b62ee-52ad-409c-8e60-14836dc94083)
--
-- Idempotent: DELETE + INSERT ON CONFLICT DO UPDATE.

BEGIN;

-- 1. Drop placeholder ARCH rows from migration 0010
DELETE FROM ssot.scarab_strategy
 WHERE service_id IN ('arch-jepa-1', 'arch-hybrid-1', 'arch-nca-1')
    OR account IN ('phd-acc4', 'phd-acc5', 'phd-acc6');

-- 2. Re-insert with REAL Railway service_ids + canonical ACC* accounts
INSERT INTO ssot.scarab_strategy
  (service_id, account, optimizer, format, hidden, lr, seed, steps,
   status, generation, updated_by, trainer_bin, w_jepa, w_nca)
VALUES
  -- ACC4 carrier — JEPA-T pure
  ('051cb5a4-1e62-48c8-8b02-abb14c387611', 'ACC4', 'adamw', 'gf256', 384, 0.001, 1597, 30000,
   'active', 1, 'migration-0011', 'tjepa_train', 0.15, 0.00),
  -- ACC5 carrier — HYBRID (N-gram + Attn + NCA, all aux losses)
  ('01cf32d5-cac1-45d8-b120-0ceba7a2647a', 'ACC5', 'muon',  'gf256', 384, 0.001, 2584, 30000,
   'active', 1, 'migration-0011', 'hybrid_train', 0.15, 0.10),
  -- ACC6 carrier — NCA pure (entropy band only)
  ('002b62ee-52ad-409c-8e60-14836dc94083', 'ACC6', 'adamw', 'gf256', 384, 0.001, 4181, 30000,
   'active', 1, 'migration-0011', 'hybrid_train', 0.00, 0.25)
ON CONFLICT (service_id) DO UPDATE SET
  account     = EXCLUDED.account,
  optimizer   = EXCLUDED.optimizer,
  format      = EXCLUDED.format,
  hidden      = EXCLUDED.hidden,
  lr          = EXCLUDED.lr,
  seed        = EXCLUDED.seed,
  steps       = EXCLUDED.steps,
  status      = EXCLUDED.status,
  generation  = EXCLUDED.generation,
  updated_by  = EXCLUDED.updated_by,
  trainer_bin = EXCLUDED.trainer_bin,
  w_jepa      = EXCLUDED.w_jepa,
  w_nca       = EXCLUDED.w_nca;

-- 3. Audit
DO $$
DECLARE
  r RECORD;
BEGIN
  FOR r IN
    SELECT service_id, account, format, hidden, lr, seed, optimizer,
           trainer_bin, w_jepa, w_nca
      FROM ssot.scarab_strategy
     WHERE trainer_bin <> 'trios-train'
     ORDER BY account
  LOOP
    RAISE NOTICE 'ARCH lane: acc=% svc=% fmt=% h=% lr=% seed=% opt=% bin=% w_jepa=% w_nca=%',
      r.account, r.service_id, r.format, r.hidden, r.lr, r.seed, r.optimizer,
      r.trainer_bin, r.w_jepa, r.w_nca;
  END LOOP;
END $$;

COMMIT;

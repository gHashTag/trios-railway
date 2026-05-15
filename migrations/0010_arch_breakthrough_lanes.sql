-- Migration 0010 — ARCH-BREAKTHROUGH lanes (JEPA-T + NCA + Hybrid)
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP · DOI 10.5281/zenodo.19227877
--
-- Context: Champion plateau IGLA-DEEP-CHAMPION-gf256-h384-LR0.0001-rng1597-adamw
-- frozen at BPB=2.5719 for 16h+ (Gate-2 needs <2.50, Δ=+0.0719). Format/algo/LR
-- sweeps alone cannot break it — ARCHITECTURE must enter the race.
--
-- trios-trainer-igla PR #152 unlocks two trainer binaries that have existed in
-- the repo since EPIC #446 but were blocked by the Dockerfile build list +
-- entrypoint allowlist:
--   - tjepa_train  → ArchKind::Jepa     (multi-objective: NTP + JEPA predictive)
--   - hybrid_train → ArchKind::Hybrid   (N-gram + HybridAttn + NCA entropy band)
--
-- This migration adds three orthogonal axes to ssot.scarab_strategy so the
-- Sovereign Scarab loop can dispatch ARCH lanes onto the fleet:
--   trainer_bin TEXT  — DEFAULT 'trios-train' (preserves existing behaviour)
--   w_jepa      NUMERIC DEFAULT 0   — JEPA predictive loss weight
--   w_nca       NUMERIC DEFAULT 0   — NCA entropy-band loss weight
--
-- Canonical ARCH lane strategies (canon_name pattern preserved):
--   IGLA-ARCH-JEPA-gf256-h384-LR0.001-rng1597-adamw    w_CE=1.0 w_JEPA=0.15
--   IGLA-ARCH-HYBRID-gf256-h384-LR0.001-rng1597-muon   w_CE=1.0 w_JEPA=0.15 w_NCA=0.10
--   IGLA-ARCH-NCA-gf256-h384-LR0.001-rng1597-adamw     w_CE=1.0 w_NCA=0.25
--
-- All seeds Fibonacci/Lucas-canon (1597, 2584, 4181). All hidden ≥256 (R6).
-- All LR ≤1e-3 on gf256 (matches DEEP-CHAMPION baseline).
-- Idempotent: ADD COLUMN IF NOT EXISTS + ON CONFLICT DO NOTHING.

BEGIN;

-- 1. Axis extension --------------------------------------------------------

ALTER TABLE ssot.scarab_strategy
  ADD COLUMN IF NOT EXISTS trainer_bin TEXT NOT NULL DEFAULT 'trios-train',
  ADD COLUMN IF NOT EXISTS w_jepa      NUMERIC NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS w_nca       NUMERIC NOT NULL DEFAULT 0;

-- Whitelist: only known binaries from trios-trainer-igla entrypoint allowlist
ALTER TABLE ssot.scarab_strategy
  DROP CONSTRAINT IF EXISTS scarab_strategy_trainer_bin_chk;

ALTER TABLE ssot.scarab_strategy
  ADD CONSTRAINT scarab_strategy_trainer_bin_chk
  CHECK (trainer_bin IN ('trios-train', 'scarab', 'gf16_test', 'ngram_train_gf16',
                          'tjepa_train', 'hybrid_train'));

-- Weights must be non-negative and reasonable (sanity bound)
ALTER TABLE ssot.scarab_strategy
  DROP CONSTRAINT IF EXISTS scarab_strategy_weights_chk;

ALTER TABLE ssot.scarab_strategy
  ADD CONSTRAINT scarab_strategy_weights_chk
  CHECK (w_jepa >= 0 AND w_jepa <= 1.0 AND w_nca >= 0 AND w_nca <= 1.0);

-- 2. Insert ARCH-BREAKTHROUGH lanes (slots arch-1..arch-3 on ACC4) ---------
-- ACC4 = phd-acc4 project (slots wave-* candidates for kill to free capacity)
-- service_id slots are placeholders; mass-deploy-arch.yml will assign Railway IDs.

INSERT INTO ssot.scarab_strategy
  (service_id, account, optimizer, format, hidden, lr, seed, steps,
   status, generation, updated_by, trainer_bin, w_jepa, w_nca)
VALUES
  ('arch-jepa-1',   'phd-acc4', 'adamw', 'gf256', 384, 0.001, 1597, 30000,
   'active', 1, 'migration-0010', 'tjepa_train', 0.15, 0.00),
  ('arch-hybrid-1', 'phd-acc5', 'muon',  'gf256', 384, 0.001, 2584, 30000,
   'active', 1, 'migration-0010', 'hybrid_train', 0.15, 0.10),
  ('arch-nca-1',    'phd-acc6', 'adamw', 'gf256', 384, 0.001, 4181, 30000,
   'active', 1, 'migration-0010', 'hybrid_train', 0.00, 0.25)
ON CONFLICT (service_id) DO NOTHING;

-- 3. Audit ----------------------------------------------------------------

DO $$
DECLARE
  arch_rows int;
BEGIN
  SELECT COUNT(*) INTO arch_rows
    FROM ssot.scarab_strategy
   WHERE trainer_bin IN ('tjepa_train', 'hybrid_train');
  RAISE NOTICE 'Migration 0010 — ARCH lanes registered: %', arch_rows;
END $$;

DO $$
DECLARE
  r RECORD;
BEGIN
  FOR r IN
    SELECT service_id, format, hidden, lr, seed, optimizer,
           trainer_bin, w_jepa, w_nca
      FROM ssot.scarab_strategy
     WHERE status = 'active' AND trainer_bin <> 'trios-train'
     ORDER BY service_id
  LOOP
    RAISE NOTICE 'ARCH strategy: svc=% fmt=% h=% lr=% seed=% opt=% bin=% w_jepa=% w_nca=%',
      r.service_id, r.format, r.hidden, r.lr, r.seed, r.optimizer,
      r.trainer_bin, r.w_jepa, r.w_nca;
  END LOOP;
END $$;

COMMIT;

-- Migration 0009 — INT4/INT8 LR rescue (gradient explosion @ LR=1e-4)
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP · DOI 10.5281/zenodo.19227877
--
-- Context: After migration 0008 applied 2026-05-15T08:55Z, 11 new STRATEGY
-- formats began writing live BPB. R5 probe at 09:32Z shows int4/int8 BPB
-- diverged from ~7.0 (uniform-init) to ~32.9 at step > 50k — classic
-- gradient explosion under aggressive low-bit quantization with LR=1e-4.
--
-- Comparable working baselines on the same trainer/seed:
--   gf4 (4-bit Galois)  @ LR=1e-4  → BPB 7.01 (stable, no explosion)
--   gf256 (8-bit Galois) @ LR=1e-4 → BPB 2.59 (champion, fully training)
--   posit8 (8-bit posit) @ LR=3e-4 → BPB 4.82 (training, was stuck at 6.97 before LR bump)
--
-- This migration drops int4 + int8 LR from 1e-4 to 1e-5 to break out of
-- the explosion while preserving the same hidden, seed, optimizer triplet
-- so the PhD MATRIX cell (format, algo) remains comparable. After redeploy,
-- expect convergence to BPB < 5.0 within 30k steps based on gf8 reference.
--
-- Idempotent: WHERE clauses guard exact rows.

DO $$
DECLARE
  int4_rows int;
  int8_rows int;
BEGIN
  UPDATE ssot.scarab_strategy
     SET lr = '0.00001'
   WHERE status = 'active'
     AND format = 'int4'
     AND lr = '0.0001';
  GET DIAGNOSTICS int4_rows = ROW_COUNT;

  UPDATE ssot.scarab_strategy
     SET lr = '0.00001'
   WHERE status = 'active'
     AND format = 'int8'
     AND lr = '0.0001';
  GET DIAGNOSTICS int8_rows = ROW_COUNT;

  RAISE NOTICE 'Migration 0009 — int4 rows updated: %, int8 rows updated: %', int4_rows, int8_rows;
END $$;

-- Audit: emit current int4/int8 strategy state
DO $$
DECLARE
  r RECORD;
BEGIN
  FOR r IN
    SELECT format, account, hidden, lr, seed, optimizer
      FROM ssot.scarab_strategy
     WHERE status = 'active' AND format IN ('int4','int8')
     ORDER BY format
  LOOP
    RAISE NOTICE 'STRATEGY rescue: format=% account=% hidden=% lr=% seed=% optimizer=%',
      r.format, r.account, r.hidden, r.lr, r.seed, r.optimizer;
  END LOOP;
END $$;

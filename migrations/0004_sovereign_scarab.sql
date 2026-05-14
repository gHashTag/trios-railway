-- Migration: ssot.scarab_strategy + ssot.scarab_heartbeat (SOVEREIGN SCARAB)
-- Implements ADR-CHAT-011 (gHashTag/trios#785) and companion crate
-- gHashTag/trios-trainer-igla#143 (scarab-pull-loop/).
--
-- Eliminates the Railway-GraphQL command surface from the scarab fleet.
-- Queen-Hive commands become SQL UPDATEs; scarabs poll their own strategy row.
--
-- R5-prototype verified locally on Postgres 17.9 (2026-05-14T09:42Z):
-- evidence at /home/user/workspace/cron_tracking/sovereign_scarab/evidence/.
--
-- Anchor: phi^2 + phi^-2 = 3 · TRINITY · DEFENSE 2026-06-15

CREATE SCHEMA IF NOT EXISTS ssot;

CREATE TABLE IF NOT EXISTS ssot.scarab_strategy (
  service_id  text PRIMARY KEY,           -- 'igla-1', 'matrix-runner-acc2-19', 'local-A'
  account     text NOT NULL,              -- 'acc0' | 'acc1' | 'acc2' | 'local'
  optimizer   text NOT NULL CHECK (optimizer IN ('adamw','muon','muon-cwd')),
  format      text NOT NULL,              -- fp32|fp16|bf16|gf16|fp8_e4m3|fp8_e5m2|int4|int8|nf4|posit16|fp80
  hidden      int  NOT NULL CHECK (hidden > 0),
  lr          numeric NOT NULL CHECK (lr > 0),
  seed        int  NOT NULL,              -- Fibonacci/Lucas waves; legacy SCARAB {47,89,123,144}
  steps       int  NOT NULL CHECK (steps > 0),
  status      text NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','stop')),
  generation  bigint NOT NULL DEFAULT 1,  -- bump → scarab respawns trainer
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  text                        -- 'queen-hive' | 'operator' | 'gardener'
);

CREATE TABLE IF NOT EXISTS ssot.scarab_heartbeat (
  service_id   text PRIMARY KEY,
  last_seen    timestamptz NOT NULL DEFAULT now(),
  current_gen  bigint NOT NULL,
  current_step int,
  current_bpb  double precision,
  pid          int,
  started_at   timestamptz
);

CREATE INDEX IF NOT EXISTS scarab_heartbeat_stale
  ON ssot.scarab_heartbeat (last_seen);

-- Atomic strategy update + generation bump. The only thing scarabs
-- ever check is generation, so the helper guarantees a single transaction
-- with `generation = generation + 1` after any field change.
CREATE OR REPLACE FUNCTION ssot.bump_strategy(
  p_service_id text,
  p_optimizer  text DEFAULT NULL,
  p_format     text DEFAULT NULL,
  p_hidden     int  DEFAULT NULL,
  p_lr         numeric DEFAULT NULL,
  p_seed       int  DEFAULT NULL,
  p_steps      int  DEFAULT NULL,
  p_status     text DEFAULT NULL,
  p_by         text DEFAULT 'queen-hive'
) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE new_gen bigint;
BEGIN
  UPDATE ssot.scarab_strategy SET
    optimizer  = COALESCE(p_optimizer, optimizer),
    format     = COALESCE(p_format, format),
    hidden     = COALESCE(p_hidden, hidden),
    lr         = COALESCE(p_lr, lr),
    seed       = COALESCE(p_seed, seed),
    steps      = COALESCE(p_steps, steps),
    status     = COALESCE(p_status, status),
    generation = generation + 1,
    updated_at = now(),
    updated_by = p_by
  WHERE service_id = p_service_id
  RETURNING generation INTO new_gen;
  RETURN new_gen;
END $$;

-- Dead-scarab detector view: any service with no heartbeat or > 2 min stale.
CREATE OR REPLACE VIEW ssot.scarab_dead AS
SELECT s.service_id, s.account, s.optimizer, s.format, s.generation AS desired_gen,
       h.last_seen, h.current_gen,
       EXTRACT(EPOCH FROM (now() - COALESCE(h.last_seen, '-infinity'::timestamptz)))/60
         AS stale_min
FROM   ssot.scarab_strategy s
LEFT JOIN ssot.scarab_heartbeat h USING (service_id)
WHERE  h.last_seen IS NULL OR h.last_seen < now() - interval '2 minutes';

-- After this migration lands, the trios-trainer-igla#143 image becomes
-- safe to deploy as the new sovereign scarab. Old Dockerfile.scarab
-- remains for the A/B control group during R5 of the migration.

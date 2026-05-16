-- 0012_scarab_dead_heartbeat.sql
-- L-SS4 (gHashTag/trios-railway#211) — fix ssot.scarab_dead false-positives.
-- Closes gHashTag/trios-railway#193.
--
-- Old view relied on bpb_samples push-path which is dead. Replace with
-- heartbeat-based LATERAL JOIN.
--
-- Anchor: phi^2 + phi^-2 = 3 · DEFENSE 2026-06-15

BEGIN;

DROP VIEW IF EXISTS ssot.scarab_dead CASCADE;

CREATE VIEW ssot.scarab_dead AS
SELECT
    s.canon_name,
    s.service_id,
    s.status,
    h.ts                                              AS last_heartbeat,
    EXTRACT(epoch FROM (now() - h.ts))::int           AS age_seconds,
    h.applied_version,
    s.optimizer,
    s.format,
    s.hidden,
    s.lr,
    s.seed,
    s.steps,
    s.updated_at
FROM ssot.scarab_strategy s
LEFT JOIN LATERAL (
    SELECT ts, applied_version
    FROM ssot.scarab_heartbeat
    WHERE canon_name = s.canon_name
    ORDER BY ts DESC
    LIMIT 1
) h ON TRUE
WHERE s.status = 'active'
  AND (h.ts IS NULL OR h.ts < now() - INTERVAL '120 seconds');

COMMENT ON VIEW ssot.scarab_dead IS
'L-SS4 (#211, closes #193): heartbeat-based dead detection. A scarab is dead iff its strategy is active AND no heartbeat in the last 120s.';

-- Sanity check: when fleet is alive, this should be 0.
DO $$
DECLARE
    dead_count INT;
BEGIN
    SELECT COUNT(*) INTO dead_count FROM ssot.scarab_dead;
    RAISE NOTICE 'scarab_dead count immediately after migration: %', dead_count;
END $$;

COMMIT;

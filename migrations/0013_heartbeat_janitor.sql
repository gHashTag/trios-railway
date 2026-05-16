-- 0013_heartbeat_janitor.sql
-- L-SS5 (gHashTag/trios-railway#212) — Sovereign Scarab v4 heartbeat-TTL janitor.
-- Closes gHashTag/trios-trainer-igla#89. Epic: gHashTag/trios#940.
--
-- Adds:
--   1. ssot.janitor_release_stale()  — advisory-lock-guarded cleanup function
--   2. ssot.janitor_status           — observability view: lock holder + hourly stats
--
-- Advisory-lock contract (G-SS-10):
--   Lock key = 0xDEAD_BEEF = 3735928559
--   Only one Postgres session may run the janitor at a time.
--   pg_try_advisory_lock() is used (non-blocking); if another session already
--   holds the lock this invocation is a no-op and returns 0.
--   Lock is released explicitly via pg_advisory_unlock() in the EXCEPTION block
--   and in the normal path, so the session-level lock is never leaked.
--
-- Anchor: phi^2 + phi^-2 = 3 · DEFENSE 2026-06-15

BEGIN;

-- ─────────────────────────────────────────────────────────────────────────────
-- 1. janitor_release_stale()
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION ssot.janitor_release_stale()
RETURNS integer
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    _lock_key  bigint  := x'DEADBEEF'::bigint;  -- 3735928559
    _got_lock  boolean;
    _released  integer := 0;
BEGIN
    -- Try to acquire session-level advisory lock (non-blocking).
    -- Contract G-SS-10: only one janitor runs at a time across the fleet.
    _got_lock := pg_try_advisory_lock(_lock_key);

    IF NOT _got_lock THEN
        RAISE NOTICE 'janitor_release_stale: advisory lock 0xDEADBEEF already held by another session — skipping';
        RETURN 0;
    END IF;

    BEGIN
        -- Release scarabs that have been dead (no heartbeat) for > 600 seconds.
        -- ssot.scarab_dead is the canonical source-of-truth view (L-SS4, #211).
        UPDATE ssot.scarab_strategy
        SET    status = 'released'
        WHERE  canon_name IN (
            SELECT canon_name
            FROM   ssot.scarab_dead
            WHERE  age_seconds > 600
        );

        GET DIAGNOSTICS _released = ROW_COUNT;

        RAISE NOTICE 'janitor_release_stale: released % stale scarab(s)', _released;

        -- Release advisory lock — normal path.
        PERFORM pg_advisory_unlock(_lock_key);

    EXCEPTION WHEN OTHERS THEN
        -- Always release the lock even on error to avoid session-level leak.
        PERFORM pg_advisory_unlock(_lock_key);
        RAISE;
    END;

    RETURN _released;
END;
$$;

COMMENT ON FUNCTION ssot.janitor_release_stale() IS
'L-SS5 (#212, closes gHashTag/trios-trainer-igla#89, epic gHashTag/trios#940): '
'Heartbeat-TTL janitor. Uses advisory lock 0xDEADBEEF (3735928559) per G-SS-10 contract '
'to ensure at-most-one execution across the fleet. '
'Releases scarabs whose last heartbeat is older than 600 seconds. '
'Returns the count of rows transitioned to status=''released''. '
'Non-blocking: returns 0 immediately if lock is already held.';

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. janitor_status view
-- ─────────────────────────────────────────────────────────────────────────────

CREATE OR REPLACE VIEW ssot.janitor_status AS
SELECT
    -- Current advisory-lock holder (NULL when lock is free)
    lock_info.pid                                           AS lock_holder_pid,
    lock_info.usename                                       AS lock_holder_user,
    lock_info.application_name                             AS lock_holder_app,
    lock_info.state                                        AS lock_holder_state,
    lock_info.query_start                                  AS lock_holder_query_start,

    -- How many scarabs were released by the janitor in the last hour
    -- We detect this via status='released' AND updated_at within the last hour.
    -- (updated_at is set by a trigger or explicit SET; relies on ssot.scarab_strategy.updated_at
    --  being maintained — same assumption as the rest of the SSOT layer.)
    COALESCE(hourly.released_count, 0)                     AS released_last_hour,

    -- Current dead-but-not-yet-released count (candidates for next run)
    COALESCE(pending.pending_count, 0)                     AS pending_release_count,

    now()                                                  AS observed_at

FROM
    -- Lock holder sub-query: join pg_locks → pg_stat_activity
    (
        SELECT
            a.pid,
            a.usename,
            a.application_name,
            a.state,
            a.query_start
        FROM pg_locks l
        JOIN pg_stat_activity a ON a.pid = l.pid
        WHERE l.locktype    = 'advisory'
          AND l.classid     = (x'DEADBEEF'::bigint >> 32)::int   -- high 32 bits
          AND l.objid       = (x'DEADBEEF'::bigint & x'FFFFFFFF'::bigint)::int  -- low 32 bits
          AND l.granted     = true
        LIMIT 1
    ) lock_info

    CROSS JOIN LATERAL (
        SELECT COUNT(*) AS released_count
        FROM ssot.scarab_strategy
        WHERE status     = 'released'
          AND updated_at >= now() - INTERVAL '1 hour'
    ) hourly

    CROSS JOIN LATERAL (
        SELECT COUNT(*) AS pending_count
        FROM ssot.scarab_dead
        WHERE age_seconds > 600
    ) pending;

COMMENT ON VIEW ssot.janitor_status IS
'L-SS5 (#212): Observability view for the heartbeat-TTL janitor. '
'Shows: current advisory-lock holder (pid/user/app), count of scarabs released in the last hour, '
'and count of stale scarabs pending release on next janitor run.';

-- ─────────────────────────────────────────────────────────────────────────────
-- Sanity check
-- ─────────────────────────────────────────────────────────────────────────────
DO $$
DECLARE
    _pending int;
BEGIN
    SELECT pending_release_count INTO _pending FROM ssot.janitor_status;
    RAISE NOTICE 'janitor_status.pending_release_count immediately after migration: %', _pending;
END $$;

COMMIT;

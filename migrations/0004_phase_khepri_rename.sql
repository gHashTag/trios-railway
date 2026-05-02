-- 0004: Phase Khepri-1 Rename Tables
-- Renames experiment_queue → strategy_queue and workers → scarabs
-- Part of Phase Khepri-1: Stateless fungible worker pool
-- All statements are idempotent (safe to run multiple times)

-- Rename experiment_queue → strategy_queue
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'experiment_queue') THEN
        ALTER TABLE experiment_queue RENAME TO strategy_queue;
    END IF;
END $$;

-- Rename workers → scarabs
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'workers') THEN
        ALTER TABLE workers RENAME TO scarabs;
    END IF;
END $$;

-- Update index names to match new table names
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'experiment_queue_pull_idx') THEN
        ALTER INDEX experiment_queue_pull_idx RENAME TO strategy_queue_pull_idx;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'experiment_queue_canon_idx') THEN
        ALTER INDEX experiment_queue_canon_idx RENAME TO strategy_queue_canon_idx;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'experiment_queue_stale_claim_idx') THEN
        ALTER INDEX experiment_queue_stale_claim_idx RENAME TO strategy_queue_stale_claim_idx;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'workers_heartbeat_idx') THEN
        ALTER INDEX workers_heartbeat_idx RENAME TO scarabs_heartbeat_idx;
    END IF;
END $$;

-- Update trigger to use new table name
DROP TRIGGER IF EXISTS trg_experiment_pending ON strategy_queue;
CREATE TRIGGER trg_strategy_pending
AFTER INSERT OR UPDATE OF status ON strategy_queue
FOR EACH ROW WHEN (NEW.status = 'pending')
EXECUTE FUNCTION notify_experiment_pending();

COMMENT ON TRIGGER trg_strategy_pending ON strategy_queue IS
'Khepri-1: NOTIFY workers when new experiments become pending for instant wake-up';

-- Update partial index for fast claim (fixes migration 002 reference)
DROP INDEX IF EXISTS idx_experiment_queue_pending;
CREATE INDEX IF NOT EXISTS idx_strategy_queue_pending
ON strategy_queue (priority DESC, id ASC)
WHERE status = 'pending';

-- Update v_leaderboard to use strategy_queue
DROP VIEW IF EXISTS v_leaderboard;
CREATE OR REPLACE VIEW v_leaderboard AS
    SELECT
        q.canon_name,
        q.seed,
        q.account,
        q.status,
        q.priority,
        COALESCE(b.best_bpb, q.final_bpb) AS best_bpb,
        b.last_step,
        b.last_ts,
        q.created_at,
        q.finished_at
    FROM strategy_queue q
    LEFT JOIN (
        SELECT canon_name, seed,
               MIN(bpb) AS best_bpb,
               MAX(step) AS last_step,
               MAX(ts)   AS last_ts
        FROM bpb_samples
        GROUP BY canon_name, seed
    ) b ON b.canon_name = q.canon_name AND b.seed = q.seed;

COMMENT ON VIEW v_leaderboard IS
'Khepri-1: Live leaderboard using strategy_queue table';

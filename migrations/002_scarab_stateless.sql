-- Migration 002: Stateless Scarab Pool (Phase Khepri-1)
-- Adds NOTIFY trigger for real-time worker wake-up and index for fast claims
-- Workers are now fungible — any worker picks any pending experiment

-- Index for fast claim: priority DESC, id ASC (matching CLAIM_SQL ordering)
CREATE INDEX IF NOT EXISTS idx_experiment_queue_pending
ON experiment_queue (priority DESC, id ASC)
WHERE status = 'pending';

-- Function to notify workers when a new experiment becomes pending
CREATE OR REPLACE FUNCTION notify_experiment_pending() RETURNS trigger AS $$
BEGIN
    -- Fire notification on INSERT (new experiment) or UPDATE (status→pending)
    PERFORM pg_notify('experiment_pending', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger: fire on INSERT or when status changes TO 'pending'
DROP TRIGGER IF EXISTS trg_experiment_pending ON experiment_queue;
CREATE TRIGGER trg_experiment_pending
AFTER INSERT OR UPDATE OF status ON experiment_queue
FOR EACH ROW WHEN (NEW.status = 'pending')
EXECUTE FUNCTION notify_experiment_pending();

-- Comment: workers can LISTEN 'experiment_pending' and wake up instantly
COMMENT ON TRIGGER trg_experiment_pending ON experiment_queue IS
'Khepri-1: NOTIFY workers when new experiments become pending for instant wake-up';

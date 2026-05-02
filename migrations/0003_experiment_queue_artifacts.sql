-- 0003: Add artifact columns to experiment_queue
-- For checkpoint tracking and file size verification
-- Apply via: psql "$NEON_DATABASE_URL" -f migrations/0003_experiment_queue.sql

-- Fix database name for migration
SET client_encoding TO 'UTF8';
ALTER TABLE experiment_queue
  ADD COLUMN IF NOT EXISTS artifact_path text,
  ADD COLUMN IF NOT EXISTS artifact_bytes bigint,
  ADD COLUMN IF NOT EXISTS artifact_sha256 text;

-- Index for faster artifact lookups
CREATE INDEX IF NOT EXISTS experiment_queue_artifact_path_idx
  ON experiment_queue (artifact_path);

-- Index for efficient size filtering
CREATE INDEX IF NOT EXISTS experiment_queue_artifact_bytes_idx
  ON experiment_queue (artifact_bytes);

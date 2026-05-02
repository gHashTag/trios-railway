-- P0: Fix timeout for FULL81K runs (14400s = 4h)
-- Execute manually on Neon when ready
-- DO NOT run via agent -- requires user confirmation

UPDATE strategy_queue
SET timeout_seconds = 14400
WHERE canon_name LIKE 'IGLA-CHAMPION-FULL81K-%'
  AND status IN ('pending', 'running');

-- Verify change
SELECT canon_name, status, timeout_seconds
FROM strategy_queue
WHERE canon_name LIKE 'IGLA-CHAMPION-FULL81K-%'
ORDER BY claimed_at DESC;

-- ──────────────────────────────────────────────────────────────────────────────
-- Corpus Inventory SQL — Classify ALL runs by corpus type
-- ──────────────────────────────────────────────────────────────────────────────
-- Purpose: Forensic audit to determine which BPB numbers are on which corpus
-- Critical for: trios#442 withdrawal decision, Gate-2 re-ratification
-- ──────────────────────────────────────────────────────────────────────────────

-- Query 1: All runs with explicit corpus tag in config_json
SELECT
    canon_name,
    config_json->'data'->>'corpus' AS corpus,
    COUNT(*) AS run_count,
    MIN(final_bpb) AS min_bpb,
    MAX(final_bpb) AS max_bpb,
    AVG(final_bpb) AS avg_bpb,
    MIN(created_at) AS first_run,
    MAX(created_at) AS last_run
FROM strategy_queue
WHERE config_json::text LIKE '%corpus%'
GROUP BY canon_name, corpus
ORDER BY first_run DESC;

-- Query 2: All runs WITHOUT explicit corpus tag (inferred from defaults)
SELECT
    'INFERRED' AS corpus_type,
    COUNT(*) AS run_count,
    'Uses entrypoint.rs defaults (tiny_shakespeare)' AS note
FROM strategy_queue
WHERE config_json::text NOT LIKE '%corpus%';

-- Query 3: Champion-specific check — fix-verify-s43 corpus confirmation
SELECT
    canon_name,
    config_json->'data'->>'corpus' AS corpus,
    config_json->'data'->>'train_path' AS train_path,
    config_json->'data'->>'val_path' AS val_path,
    status,
    final_bpb,
    created_at
FROM strategy_queue
WHERE canon_name LIKE '%fix-verify-s43%'
   OR canon_name LIKE '%CHAMPION%'
   OR canon_name LIKE '%IGLA-CHAMPION%'
ORDER BY created_at DESC;

-- Query 4: MEGA-ASHA-R2 wave check (all runs in this wave)
SELECT
    canon_name,
    config_json->'data'->>'corpus' AS corpus,
    status,
    final_bpb,
    created_at
FROM strategy_queue
WHERE canon_name LIKE '%MEGA-ASHA%'
ORDER BY created_at DESC;

-- Query 5: Summary by corpus type
SELECT
    COALESCE(config_json->'data'->>'corpus', 'MISSING/DEFAULT') AS corpus,
    COUNT(*) AS run_count,
    COUNT(DISTINCT canon_name) AS unique_canons,
    COUNT(CASE WHEN status = 'done' THEN 1 END) AS completed_runs,
    MIN(CASE WHEN status = 'done' THEN final_bpb END) AS min_done_bpb,
    MAX(CASE WHEN status = 'done' THEN final_bpb END) AS max_done_bpb
FROM strategy_queue
GROUP BY corpus
ORDER BY run_count DESC;

-- ──────────────────────────────────────────────────────────────────────────────
-- Expected Outcome (based on scarab-acc0 logs):
-- - corpus='tiny_shakespeare': All MEGA-ASHA-R2, fix-verify-s43, IGLA-CHAMPION runs
-- - corpus='fineweb' or MISSING: Zero rows (Bug C — no fineweb runs exist)
-- - Default (entrypoint.rs): tiny_shakespeare.txt
-- ──────────────────────────────────────────────────────────────────────────────

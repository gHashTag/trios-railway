-- Forensic inventory: classify all strategy_queue runs by corpus
-- This will determine which experiments ran on tiny_shakespeare vs fineweb
-- Based on config_json::text LIKE '%corpus%' pattern

SELECT
    canon_name,
    config_json::jsonb->'data'->>'corpus' AS corpus,
    COUNT(*) AS run_count,
    MIN(final_bpb) AS min_bpb,
    MAX(final_bpb) AS max_bpb,
    AVG(final_bpb) AS avg_bpb,
    MIN(steps_budget) AS min_steps,
    MAX(steps_budget) AS max_steps,
    MIN(started_at) AS first_run,
    MAX(started_at) AS last_run
FROM strategy_queue
WHERE config_json::text LIKE '%corpus%'
GROUP BY corpus, canon_name
ORDER BY corpus DESC, run_count DESC;

-- Also check: how many runs have NO corpus specification
SELECT COUNT(*) AS no_corpus_spec_count,
       COUNT(DISTINCT canon_name) AS distinct_canons
FROM strategy_queue
WHERE config_json::text NOT LIKE '%corpus%';

-- And: MEGA-ASHA-R2 runs specifically (fix-verify-s43 architecture)
SELECT id,
       canon_name,
       config_json::jsonb->'data'->>'corpus' AS corpus,
       steps_budget,
       final_bpb,
       started_at,
       finished_at,
       status
FROM strategy_queue
WHERE canon_name LIKE '%MEGA-ASHA-R2%' OR canon_name LIKE '%fix-verify%'
ORDER BY started_at DESC
LIMIT 20;

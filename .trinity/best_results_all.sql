-- IGLA RACE Best Results Query — по всем форматы×архитектурам×алгоритмам
-- Комплексный отчёт для поиска лучших конфигураций

-- === 1. ЛУЧШИЕ РЕЗУЛЬТАТЫ ПО КАЖДОЙ КОМБИНАЦИИ ===

-- Best overall (Gate-2 candidates)
WITH gate2_candidates AS (
    SELECT
        canon_name,
        config_json->>'format' as format,
        config_json->>'arch' as arch,
        config_json->>'optimizer' as optimizer,
        config_json->>'hidden'::int as hidden,
        final_bpb,
        final_step,
        account,
        created_at,
        ROW_NUMBER() OVER (ORDER BY final_bpb ASC) as rank
    FROM experiment_queue
    WHERE status = 'done'
      AND final_bpb < 1.85  -- Gate-2 threshold
      AND final_step >= 500     -- minimum steps
      AND config_json NOT LIKE '%mock%'
    ORDER BY final_bpb ASC
    LIMIT 20
)
SELECT * FROM gate2_candidates;

-- === 2. ЛУЧШИЕ ПО ФОРМАТУ ===

WITH best_by_format AS (
    SELECT
        config_json->>'format' as format,
        MIN(final_bpb) as best_bpb,
        MIN(final_step) as at_best_step,
        COUNT(*) as experiments_count,
        ARRAY_AGG(
            canon_name
            ORDER BY final_bpb ASC
            LIMIT 3
        ) as top_3_canons
    FROM experiment_queue
    WHERE status = 'done'
      AND final_step >= 500
      AND config_json->>'format' IS NOT NULL
    GROUP BY config_json->>'format'
)
SELECT
    format,
    best_bpb,
    at_best_step,
    experiments_count,
    top_3_canons
FROM best_by_format
ORDER BY best_bpb ASC;

-- === 3. ЛУЧШИЕ ПО АРХИТЕКТУРЕ ===

WITH best_by_arch AS (
    SELECT
        config_json->>'arch' as arch,
        MIN(final_bpb) as best_bpb,
        MIN(final_step) as at_best_step,
        COUNT(*) as experiments_count,
        ARRAY_AGG(
            canon_name
            ORDER BY final_bpb ASC
            LIMIT 3
        ) as top_3_canons
    FROM experiment_queue
    WHERE status = 'done'
      AND final_step >= 500
      AND config_json->>'arch' IS NOT NULL
    GROUP BY config_json->>'arch'
)
SELECT
    arch,
    best_bpb,
    at_best_step,
    experiments_count,
    top_3_canons
FROM best_by_arch
ORDER BY best_bpb ASC;

-- === 4. ЛУЧШИЕ ПО АЛГОРИТМУ ===

WITH best_by_optimizer AS (
    SELECT
        config_json->>'optimizer' as optimizer,
        MIN(final_bpb) as best_bpb,
        MIN(final_step) as at_best_step,
        COUNT(*) as experiments_count,
        ARRAY_AGG(
            canon_name
            ORDER BY final_bpb ASC
            LIMIT 3
        ) as top_3_canons
    FROM experiment_queue
    WHERE status = 'done'
      AND final_step >= 500
      AND config_json->>'optimizer' IS NOT NULL
    GROUP BY config_json->>'optimizer'
)
SELECT
    optimizer,
    best_bpb,
    at_best_step,
    experiments_count,
    top_3_canons
FROM best_by_optimizer
ORDER BY best_bpb ASC;

-- === 5. ПОЛНАЯ МАТРИЦА ФОРМАТ × АРХИТЕКТУРА × АЛГОРИТМ ===

WITH matrix_summary AS (
    SELECT
        COALESCE(config_json->>'format', 'unknown') as format,
        COALESCE(config_json->>'arch', 'unknown') as arch,
        COALESCE(config_json->>'optimizer', 'unknown') as optimizer,
        COUNT(*) as total_experiments,
        COUNT(*) FILTER (WHERE final_step >= 500) as valid_experiments,
        MIN(final_bpb) as best_bpb,
        AVG(final_bpb) as avg_bpb,
        STDDEV(final_bpb) as std_bpb,
        MAX(final_step) as max_step,
        AVG(final_step) as avg_step
    FROM experiment_queue
    WHERE status = 'done'
      AND config_json NOT LIKE '%mock%'
    GROUP BY GROUPING SETS (config_json->>'format', config_json->>'arch', config_json->>'optimizer')
)
SELECT
    format,
    arch,
    optimizer,
    total_experiments,
    valid_experiments,
    best_bpb,
    avg_bpb,
    std_bpb,
    max_step,
    avg_step,
    -- Сравнение с Gate-2
    (best_bpb - 1.85)::numeric(10,3) as gap_to_gate2,
    CASE
        WHEN best_bpb < 1.85 THEN '✅ PASS'
        WHEN best_bpb < 2.0 THEN '⚠️ CLOSE'
        WHEN best_bpb < 3.0 THEN '⏳ WARMUP'
        ELSE '❌ FAIL'
    END as gate2_status
FROM matrix_summary
ORDER BY format, arch, optimizer, best_bpb ASC;

-- === 6. PHI ФОРМАТЫ (GF4, GF8, GF12, GF16, GF20, GF24, GF32, GF64) ===

WITH phi_formats AS (
    SELECT
        canon_name,
        config_json->>'format' as phi_format,
        config_json->>'arch' as arch,
        final_bpb,
        final_step,
        created_at
    FROM experiment_queue
    WHERE status = 'done'
      AND config_json->>'format' IN ('gf4', 'gf8', 'gf12', 'gf16', 'gf20', 'gf24', 'gf32', 'gf64')
      AND config_json NOT LIKE '%mock%'
      AND final_step >= 500
)
SELECT
    phi_format,
    arch,
    final_bpb,
    final_step,
    created_at,
    -- Phi-distance (меньше = лучше)
    CASE
        WHEN phi_format = 'gf16' THEN 0.0      -- φ-distance = 0 (reference)
        WHEN phi_format = 'gf8' THEN 0.3819    -- φ-distance = φ⁻² ≈ 0.382
        WHEN phi_format = 'gf12' THEN 0.0902   -- φ-distance = φ⁻⁵
        WHEN phi_format = 'gf4' THEN 0.1459    -- φ-distance = φ⁻⁴
        WHEN phi_format = 'gf20' THEN 0.0557    -- φ-distance = φ⁻⁶
        WHEN phi_format = 'gf24' THEN 0.0344    -- φ-distance = φ⁻⁷
        WHEN phi_format = 'gf32' THEN 0.2361    -- φ-distance = φ⁻³
        WHEN phi_format = 'gf64' THEN 2.6180    -- φ-distance = φ
        ELSE NULL
    END as phi_distance,
    CASE
        WHEN final_bpb < 1.85 THEN '✅ PASS'
        WHEN final_bpb < 2.0 THEN '⚠️ CLOSE'
        WHEN final_bpb < 3.0 THEN '⏳ WARMUP'
        ELSE '❌ FAIL'
    END as gate2_status
FROM phi_formats
ORDER BY
    CASE phi_format
        WHEN 'gf16' THEN 1
        WHEN 'gf8' THEN 2
        WHEN 'gf12' THEN 3
        WHEN 'gf4' THEN 4
        WHEN 'gf20' THEN 5
        WHEN 'gf24' THEN 6
        WHEN 'gf32' THEN 7
        WHEN 'gf64' THEN 8
        ELSE 9
    END,
    final_bpb ASC
LIMIT 20;

-- === 7. JEPA-T SPECIAL QUERY ===

WITH jepa_performance AS (
    SELECT
        canon_name,
        config_json->>'arch' as arch,
        COALESCE((config_json->>'d_model')::int, 0) as d_model,
        final_bpb,
        final_step,
        created_at
    FROM experiment_queue
    WHERE status = 'done'
      AND config_json->>'arch' = 'jepa'
      AND config_json NOT LIKE '%mock%'
      AND final_step >= 500
)
SELECT
    arch,
    AVG(d_model) as avg_d_model,
    COUNT(*) as total_experiments,
    COUNT(*) FILTER (WHERE final_bpb < 1.85) as gate2_pass,
    COUNT(*) FILTER (WHERE final_bpb < 2.0) as warmup_pass,
    MIN(final_bpb) as best_bpb,
    MIN(final_step) as at_best_step,
    ARRAY_AGG(
        canon_name
        ORDER BY final_bpb ASC
        LIMIT 3
    ) as top_3_canons
FROM jepa_performance;

-- === 8. АЛГОРИТМЫ СРАВНЕНИЕ (Muon vs AdamW) ===

WITH optimizer_comparison AS (
    SELECT
        config_json->>'optimizer' as optimizer,
        MIN(final_bpb) as best_bpb,
        AVG(final_bpb) as avg_bpb,
        STDDEV(final_bpb) as std_bpb,
        COUNT(*) as experiments_count,
        COUNT(*) FILTER (WHERE final_step >= 10000) as long_runs,
        COUNT(*) FILTER (WHERE final_step < 1000) AS short_runs
    FROM experiment_queue
    WHERE status = 'done'
      AND config_json->>'optimizer' IS NOT NULL
      AND config_json NOT LIKE '%mock%'
    GROUP BY config_json->>'optimizer'
)
SELECT
    optimizer,
    best_bpb,
    avg_bpb,
    std_bpb,
    experiments_count,
    long_runs,
    short_runs,
    CASE
        WHEN (SELECT COUNT(*) FROM optimizer_comparison o2 WHERE o2.best_bpb < optimizer_comparison.best_bpb AND o2.optimizer = 'muon')::int > 0
        THEN CONCAT(' beats AdamW by ', ROUND(((SELECT avg_bpb FROM optimizer_comparison WHERE optimizer='adamw') - (SELECT avg_bpb FROM optimizer_comparison WHERE optimizer='muon'))::numeric,4), 3), '%')
        ELSE ''
    END as muon_advantage
FROM optimizer_comparison
ORDER BY best_bpb ASC;

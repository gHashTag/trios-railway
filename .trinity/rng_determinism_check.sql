-- R5-CRITICAL: Check if rng_seed is actually wired in trainer
-- If 3 different seeds produce identical BPB → rng not used in init/data sampling

-- Check IGLA-RACE-h512-LR002-rng{42,44,4181} at step=1000
SELECT
  canon_name,
  seed,
  step,
  bpb,
  created_at
FROM bpb_samples
WHERE canon_name LIKE 'IGLA-RACE-h512-LR002-rng%'
  AND step = 1000
ORDER BY seed;

-- Count distinct BPB values (should be 3 if rng works, 1 if broken)
SELECT
  COUNT(DISTINCT bpb) as distinct_bpb_count,
  COUNT(*) as total_rows,
  CASE
    WHEN COUNT(DISTINCT bpb) = 1 THEN 'CRITICAL: rng not wired!'
    WHEN COUNT(DISTINCT bpb) = COUNT(*) THEN 'OK: each seed unique'
    ELSE 'WARNING: some collision'
  END as rng_status
FROM bpb_samples
WHERE canon_name LIKE 'IGLA-RACE-h512-LR002-rng%'
  AND step = 1000;

-- Check GATE2 5-seed quorum tightness
SELECT
  COUNT(DISTINCT seed) as seed_count,
  COUNT(*) as total_rows,
  AVG(bpb) as mean_bpb,
  STDDEV(bpb) as stddev_bpb,
  MIN(bpb) as min_bpb,
  MAX(bpb) as max_bpb
FROM bpb_samples
WHERE step = 1000
  AND created_at > NOW() - INTERVAL '2 hours';

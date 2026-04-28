-- Phase E.Hyperparameter Sweep — 32 experiments across 4 accounts
-- Goal: Explore unexplored hyperparameter directions to beat champion BPB=1.873
-- Current champion: IGLA-TRAIN_V2-FP32-CHAMP-E0053-rng42
--
-- Format: IGLA-TRAIN_V2-{FORMAT}-E{ID}-H{SIZE}-rng{SEED}
-- Additional suffixes for hyperparameter identification (e.g., -LR004)
--
-- Account distribution: 8 experiments per account (acc0, acc1, acc2, acc3)

INSERT INTO experiment_queue
    (canon_name, config_json, priority, seed, steps_budget, account, status, created_by)
VALUES
    -- ============================================================================
    -- Phase E.LR: Learning Rate Ladder (6 experiments, priority 80-90)
    -- ============================================================================

    -- acc0: 3 LR experiments
    (
        'IGLA-TRAIN_V2-FP32-E0100-H2048-rng1597-LR004',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.004,"phi_anchor":"INV-8: lr=alpha_phi/phi^3=0.004","note":"E.LR: Phi-optimized LR on champion d_model=2048. Expected faster convergence, potential BPB < 1.87"}'::jsonb,
        90, 1597, 2000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0101-H2048-rng2584-LR005',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.005,"phi_anchor":"lr=0.005 (aggressive beyond phi-optimal)","note":"E.LR: Aggressive LR test. Watch for instability. If stable, could enable faster learning."}'::jsonb,
        85, 2584, 2000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0102-H2048-rng4181-LR003',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.003,"phi_anchor":"lr=0.003 (conservative phi-adjacent)","note":"E.LR: Conservative phi-adjacent LR. More stable than 0.004, potentially better final convergence."}'::jsonb,
        85, 4181, 2000, 'acc0', 'pending', 'human'
    ),

    -- acc1: 3 LR experiments
    (
        'IGLA-TRAIN_V2-FP32-E0103-H1024-rng1597-LR004',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.004,"phi_anchor":"INV-8: lr=alpha_phi/phi^3=0.004","note":"E.LR: Phi-optimized LR on baseline d_model=1024. Compare with E0100 (H2048) for LR x capacity interaction."}'::jsonb,
        88, 1597, 2000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0104-H1024-rng2584-LR006',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.006,"phi_anchor":"lr=0.006 (very aggressive)","note":"E.LR: Very aggressive LR. High risk of divergence, but high reward if stable."}'::jsonb,
        80, 2584, 2000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0105-H1024-rng4181-LR0015',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1024,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.0015,"phi_anchor":"lr=0.0015 (ultra-conservative)","note":"E.LR: Ultra-conservative LR. Slower but potentially better final BPB through more precise convergence."}'::jsonb,
        80, 4181, 2000, 'acc1', 'pending', 'human'
    ),

    -- ============================================================================
    -- Phase E.OPT: Alternative Optimizers (6 experiments, priority 75-85)
    -- ============================================================================

    -- acc2: 3 OPT experiments
    (
        'IGLA-TRAIN_V2-FP32-E0200-H2048-rng1597-SGD',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"SGD","lr":0.01,"phi_anchor":"SGD with higher LR (no momentum)","note":"E.OPT: Pure SGD. Slower but potentially flatter minima. LR=0.01 is SGD-standard vs AdamW 0.002."}'::jsonb,
        85, 1597, 2000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0201-H2048-rng2584-SGDMOM',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"SGDMomentum","lr":0.01,"momentum":0.9,"phi_anchor":"SGD with momentum=0.9","note":"E.OPT: SGD with momentum. Combines SGD generalization with momentum acceleration. Could beat AdamW final BPB."}'::jsonb,
        85, 2584, 2000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0202-H2048-rng4181-RMSPROP',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"RMSprop","lr":0.004,"phi_anchor":"RMSprop with adaptive learning","note":"E.OPT: RMSprop. Different adaptive scheme than AdamW. May handle varying gradient scales better."}'::jsonb,
        80, 4181, 2000, 'acc2', 'pending', 'human'
    ),

    -- acc3: 3 OPT experiments
    (
        'IGLA-TRAIN_V2-FP32-E0203-H2048-rng1597-ADAM',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"Adam","lr":0.002,"phi_anchor":"Adam (no weight decay)","note":"E.OPT: Adam without weight decay. Compare to AdamW to see if weight decay is helping or hurting BPB."}'::jsonb,
        80, 1597, 2000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0204-H2048-rng2584-ADAMW-LOWWD',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"weight_decay":0.001,"phi_anchor":"AdamW with reduced weight decay","note":"E.OPT: AdamW with weight_decay=0.001 (vs default 0.01). Less regularization may help fit training data better."}'::jsonb,
        75, 2584, 2000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0205-H2048-rng4181-ADAMW-HIGHWD',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"weight_decay":0.05,"phi_anchor":"AdamW with increased weight decay","note":"E.OPT: AdamW with weight_decay=0.05. More regularization may improve generalization despite higher training loss."}'::jsonb,
        75, 4181, 2000, 'acc3', 'pending', 'human'
    ),

    -- ============================================================================
    -- Phase E.DIM: d_model Capacity Sweep (6 experiments, priority 70-85)
    -- ============================================================================

    -- acc0: 2 DIM experiments
    (
        'IGLA-TRAIN_V2-FP32-E0300-H768-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":768,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=768 (3/4 of 1024)","note":"E.DIM: Smaller than baseline. Easier to optimize, potentially better BPB if 1024 is overkill."}'::jsonb,
        80, 1597, 2000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0301-H1536-rng2584',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1536,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=1536 (1.5x baseline, phi^2=2.618 approx)","note":"E.DIM: 1.5x baseline. Near phi^2 scaling. May capture more patterns without optimization difficulty of 2048."}'::jsonb,
        80, 2584, 2000, 'acc0', 'pending', 'human'
    ),

    -- acc1: 2 DIM experiments
    (
        'IGLA-TRAIN_V2-FP32-E0302-H3072-rng4181',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":3072,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=3072 (3x baseline, phi^4 approx)","note":"E.DIM: Large model. More capacity but harder to optimize. May need more steps to beat champion."}'::jsonb,
        85, 4181, 2000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0303-H512-rng1597',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":512,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=512 (1/2 baseline, Lucas 2x256)","note":"E.DIM: Small model. Fast to train but limited capacity. May surprise with good BPB if 1024 is overparameterized."}'::jsonb,
        75, 1597, 2000, 'acc1', 'pending', 'human'
    ),

    -- acc2: 2 DIM experiments
    (
        'IGLA-TRAIN_V2-FP32-E0304-H1280-rng2584',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":1280,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=1280 (5/4 baseline, phi^2/2 approx)","note":"E.DIM: 25% larger than baseline. Near phi-adjacent scaling. Good balance of capacity and optimization."}'::jsonb,
        78, 2584, 2000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0305-H2560-rng4181',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2560,"ctx_len":12,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"d_model=2560 (2.5x baseline)","note":"E.DIM: 2.5x baseline. Between 2048 and 3072. May find sweet spot."}'::jsonb,
        80, 4181, 2000, 'acc2', 'pending', 'human'
    ),

    -- ============================================================================
    -- Phase E.CTX: Context Length Sweep (5 experiments, priority 65-75)
    -- ============================================================================

    -- acc2: 2 CTX experiments
    (
        'IGLA-TRAIN_V2-FP32-E0400-H2048-rng1597-CTX8',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":8,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"ctx_len=8 (2/3 of baseline, Lucas 2x4)","note":"E.CTX: Shorter context. Faster training, less memory. May be enough if n_gram captures most signal."}'::jsonb,
        70, 1597, 2000, 'acc2', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0401-H2048-rng2584-CTX10',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":10,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"ctx_len=10 (between 8 and 12)","note":"E.CTX: Slightly shorter than baseline. Good balance."}'::jsonb,
        72, 2584, 2000, 'acc2', 'pending', 'human'
    ),

    -- acc3: 3 CTX experiments
    (
        'IGLA-TRAIN_V2-FP32-E0402-H2048-rng4181-CTX14',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":14,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"ctx_len=14 (phi^2+2, near optimal)","note":"E.CTX: ctx_len=14, same as n_gram. May allow full n-gram window utilization. Potential sweet spot."}'::jsonb,
        74, 4181, 2000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0403-H2048-rng1597-CTX16',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":16,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"ctx_len=16 (4x4, Lucas 2x8)","note":"E.CTX: ctx_len=16, exceeds n_gram. More context for potential patterns beyond n_gram."}'::jsonb,
        75, 1597, 2000, 'acc3', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0404-H2048-rng2584-CTX20',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":20,"n_gram":14,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"ctx_len=20 (near Lucas L6)","note":"E.CTX: Longest context tested. Heavy compute but may capture longer-range dependencies."}'::jsonb,
        70, 2584, 2000, 'acc3', 'pending', 'human'
    ),

    -- ============================================================================
    -- Phase E.NGRAM: N-gram Sweep (5 experiments, priority 60-72)
    -- ============================================================================

    -- acc0: 1 NGRAM experiment
    (
        'IGLA-TRAIN_V2-FP32-E0500-H2048-rng4181-NG12',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":12,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"n_gram=12 (2x6, Lucas)","note":"E.NGRAM: n_gram=12, matches ctx_len. Simpler model, less overfitting risk."}'::jsonb,
        68, 4181, 2000, 'acc0', 'pending', 'human'
    ),

    -- acc1: 2 NGRAM experiments
    (
        'IGLA-TRAIN_V2-FP32-E0501-H2048-rng4181-NG13',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":13,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"n_gram=13 (Fibonacci F7)","note":"E.NGRAM: n_gram=13, one less than baseline. Slightly simpler."}'::jsonb,
        68, 4181, 2000, 'acc1', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0502-H2048-rng1597-NG15',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":15,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"n_gram=15 (phi^4 approx)","note":"E.NGRAM: n_gram=15, one more than baseline. Slightly more expressive."}'::jsonb,
        70, 1597, 2000, 'acc1', 'pending', 'human'
    ),

    -- acc2: 1 NGRAM experiment
    (
        'IGLA-TRAIN_V2-FP32-E0503-H2048-rng2584-NG16',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":16,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"n_gram=16 (2x8, Lucas 2xL4)","note":"E.NGRAM: n_gram=16, phi^2 approx. Near phi-optimal value."}'::jsonb,
        70, 2584, 2000, 'acc2', 'pending', 'human'
    ),

    -- acc3: 1 NGRAM experiment
    (
        'IGLA-TRAIN_V2-FP32-E0504-H2048-rng4181-NG18',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":18,"variant":"WT+resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"n_gram=18 (Lucas L6, phi^6+phi^-6)","note":"E.NGRAM: n_gram=18, largest tested. May overfit but could capture longer patterns."}'::jsonb,
        65, 4181, 2000, 'acc3', 'pending', 'human'
    ),

    -- ============================================================================
    -- Phase E.VAR: Architecture Variants (4 experiments, priority 70-80)
    -- ============================================================================

    -- acc0: 2 VAR experiments
    (
        'IGLA-TRAIN_V2-FP32-E0600-H2048-rng2584-RESIDONLY',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"variant=resid (residual only, no weight tying)","note":"E.VAR: Residual-only, no weight tying. More parameters, different optimization."}'::jsonb,
        75, 2584, 2000, 'acc0', 'pending', 'human'
    ),
    (
        'IGLA-TRAIN_V2-FP32-E0601-H2048-rng1597-WTRESID-D6',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT+resid","depth":6,"optimizer":"AdamW","lr":0.002,"phi_anchor":"variant=WT+resid with depth=6 (vs default 4)","note":"E.VAR: WT+resid with deeper architecture (6 layers). More expressiveness, harder to optimize."}'::jsonb,
        80, 1597, 2000, 'acc0', 'pending', 'human'
    ),

    -- acc1: 1 VAR experiment
    (
        'IGLA-TRAIN_V2-FP32-E0602-H2048-rng4181-NORESID',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"no-resid","optimizer":"AdamW","lr":0.002,"phi_anchor":"variant=no-resid (no residual connections)","note":"E.VAR: No residual connections. Pure feedforward, simpler gradient flow. May surprise."}'::jsonb,
        72, 4181, 2000, 'acc1', 'pending', 'human'
    ),

    -- acc3: 1 VAR experiment
    (
        'IGLA-TRAIN_V2-FP32-E0603-H2048-rng1597-WTONLY',
        '{"model":"train_v2","number_format":"fp32","s_e_m":"1:8:23","integer_type":"u32","d_model":2048,"ctx_len":12,"n_gram":14,"variant":"WT","optimizer":"AdamW","lr":0.002,"phi_anchor":"variant=WT (weight-tied only, no residual)","note":"E.VAR: WT-only, no residual connections. Simpler model, may optimize differently."}'::jsonb,
        78, 1597, 2000, 'acc3', 'pending', 'human'
    )
ON CONFLICT (canon_name, seed, account) DO NOTHING;

-- ============================================================================
-- L7 audit row
-- ============================================================================
INSERT INTO gardener_decisions (ts, action, affected_exp_ids, reason, snapshot)
SELECT
    now(),
    'enqueue',
    array_agg(id),
    'Phase E.Hyperparameter Sweep — 32 experiments across 4 accounts. Exploring LR ladder, alternative optimizers, d_model capacity, context length, n-gram, and architecture variants to beat champion BPB=1.873.',
    jsonb_build_object(
        'phase', 'E.Hyperparameter',
        'total_experiments', 32,
        'accounts', jsonb_build_object(
            'acc0', 8,
            'acc1', 8,
            'acc2', 8,
            'acc3', 8
        ),
        'phases', jsonb_build_array('E.LR', 'E.OPT', 'E.DIM', 'E.CTX', 'E.NGRAM', 'E.VAR'),
        'champion_bpb', 1.873,
        'trinity', 'phi^2 + phi^-2 = 3'
    )
FROM experiment_queue
WHERE canon_name LIKE 'E0%' AND status = 'pending';

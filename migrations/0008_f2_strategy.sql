-- 0008: F2 Protocol Schema Extension for Scarab-driven Quantization Sweeps
-- Loop 16 QQ: design-only migration (no DB execution yet).
--
-- Goal: enable F2 protocol (phi-ladder vs format-zoo) to use existing scarab
-- architecture without breaking the production scarab_strategy contract.
--
-- Strategy: encode F2-specific params (precision_bits, arm, task_kind, use_ffn)
-- in a SEPARATE companion table linked by service_id; production scarabs
-- continue using their canonical 3 formats (fp32/bf16/gf16) untouched.

-- F2 companion table — one row per F2 experiment, linked to scarab_strategy.
CREATE TABLE IF NOT EXISTS public.f2_strategy (
    service_id      TEXT PRIMARY KEY REFERENCES public.scarab_strategy(service_id) ON DELETE CASCADE,

    -- F2 arm and precision (loop 8-13 protocol)
    arm             TEXT NOT NULL CHECK (arm IN ('phi', 'zoo', 'fp32')),
    precision_bits  DOUBLE PRECISION NOT NULL CHECK (precision_bits > 0 AND precision_bits <= 32),
    quantizer       TEXT NOT NULL CHECK (quantizer IN ('paretoq_seq', 'paretoq_lsq', 'int4_rtn', 'bf16_e4m3', 'fp32_baseline')),

    -- Task and architecture (loop 16 methodology fix)
    task_kind       TEXT NOT NULL DEFAULT 'sparse_parity' CHECK (task_kind IN ('counter', 'sparse_parity', 'bytes_file')),
    task_params     JSONB,  -- e.g. {"n_bits": 40, "k": 3, "n_tasks": 64}
    use_ffn         BOOLEAN NOT NULL DEFAULT TRUE,
    d_hidden        INTEGER NOT NULL DEFAULT 64 CHECK (d_hidden > 0),

    -- iso-N_eff target (loop 12 GG)
    iso_neff_target_n  BIGINT,  -- NULL = iso-N comparison

    -- Bookkeeping
    config_fingerprint  BIGINT NOT NULL,  -- FNV-1a hash of all F2 params
    schema_version      TEXT NOT NULL DEFAULT 'f2.5',
    description         TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_f2_strategy_arm ON public.f2_strategy(arm);
CREATE INDEX IF NOT EXISTS idx_f2_strategy_fingerprint ON public.f2_strategy(config_fingerprint);

-- BPB samples extension: track which F2 experiment each sample belongs to.
-- (Assumes ssot.bpb_samples table already exists per loop 14 user description.)
-- Run this only after verifying ssot.bpb_samples columns.
COMMENT ON TABLE public.f2_strategy IS
'F2 Protocol per-scarab config. Loop 16 QQ. Links to scarab_strategy by service_id.';

-- Helper: upsert F2 strategy as companion to scarab_strategy.
-- Caller is responsible for upserting scarab_strategy first (sets format='fp32' as placeholder).
CREATE OR REPLACE FUNCTION upsert_f2_strategy(
    p_service_id     TEXT,
    p_arm            TEXT,
    p_precision_bits DOUBLE PRECISION,
    p_quantizer      TEXT,
    p_task_kind      TEXT DEFAULT 'sparse_parity',
    p_task_params    JSONB DEFAULT '{"n_bits": 40, "k": 3, "n_tasks": 64}'::jsonb,
    p_use_ffn        BOOLEAN DEFAULT TRUE,
    p_d_hidden       INTEGER DEFAULT 64,
    p_iso_neff_target_n BIGINT DEFAULT NULL,
    p_config_fingerprint BIGINT DEFAULT 0,
    p_description    TEXT DEFAULT NULL
) RETURNS void AS $$
BEGIN
    INSERT INTO public.f2_strategy (
        service_id, arm, precision_bits, quantizer, task_kind, task_params,
        use_ffn, d_hidden, iso_neff_target_n, config_fingerprint, description, last_updated
    ) VALUES (
        p_service_id, p_arm, p_precision_bits, p_quantizer, p_task_kind, p_task_params,
        p_use_ffn, p_d_hidden, p_iso_neff_target_n, p_config_fingerprint, p_description, NOW()
    )
    ON CONFLICT (service_id) DO UPDATE SET
        arm = EXCLUDED.arm,
        precision_bits = EXCLUDED.precision_bits,
        quantizer = EXCLUDED.quantizer,
        task_kind = EXCLUDED.task_kind,
        task_params = EXCLUDED.task_params,
        use_ffn = EXCLUDED.use_ffn,
        d_hidden = EXCLUDED.d_hidden,
        iso_neff_target_n = EXCLUDED.iso_neff_target_n,
        config_fingerprint = EXCLUDED.config_fingerprint,
        description = EXCLUDED.description,
        last_updated = NOW();
END;
$$ LANGUAGE plpgsql;

-- View: full F2 experiment context (joins scarab_strategy + f2_strategy + latest heartbeat).
CREATE OR REPLACE VIEW public.f2_experiment_status AS
SELECT
    s.service_id,
    s.status,
    s.seed,
    s.steps,
    s.canon_name,
    f.arm,
    f.precision_bits,
    f.quantizer,
    f.task_kind,
    f.use_ffn,
    f.iso_neff_target_n,
    f.config_fingerprint,
    h.is_training,
    h.last_heartbeat,
    f.last_updated AS f2_last_updated
FROM public.scarab_strategy s
JOIN public.f2_strategy f USING (service_id)
LEFT JOIN public.scarab_heartbeat h USING (service_id);

COMMENT ON VIEW public.f2_experiment_status IS
'Joined view of scarab + F2 config + liveness. Queen Hive reads this to monitor F2 sweeps.';

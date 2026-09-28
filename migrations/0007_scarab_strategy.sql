-- 0007: Scarab Database-Polling Architecture
-- Enables scarabs to self-manage by polling for strategy changes.
-- NO Railway API, NO PAT tokens, NO variableUpsert.

-- Primary strategy table - one row per scarab service
CREATE TABLE IF NOT EXISTS public.scarab_strategy (
    service_id      TEXT PRIMARY KEY,
    optimizer       TEXT NOT NULL CHECK (optimizer IN ('adamw', 'muon', 'muon-cwd')),
    hidden          INTEGER NOT NULL CHECK (hidden > 0),
    lr              DOUBLE PRECISION NOT NULL CHECK (lr > 0 AND lr < 1),
    seed            BIGINT NOT NULL,
    steps           INTEGER NOT NULL CHECK (steps > 0),
    ctx             INTEGER NOT NULL CHECK (ctx > 0),
    format          TEXT NOT NULL CHECK (format IN ('fp32', 'bf16', 'gf16')),
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'disabled')),
    strategy_hash   TEXT NOT NULL,
    last_updated    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    description     TEXT,
    canon_name      TEXT
);

-- Index for status filtering
CREATE INDEX IF NOT EXISTS idx_scarab_strategy_status
    ON public.scarab_strategy (status);

-- Index for heartbeat polling
CREATE INDEX IF NOT EXISTS idx_scarab_strategy_updated
    ON public.scarab_strategy (last_updated DESC);

-- Hash computation trigger
CREATE OR REPLACE FUNCTION compute_strategy_hash()
RETURNS TEXT AS $$
BEGIN
    RETURN encode(digest(
        NEW.optimizer || '|' ||
        NEW.hidden || '|' ||
        NEW.lr || '|' ||
        NEW.seed || '|' ||
        NEW.steps || '|' ||
        NEW.ctx || '|' ||
        NEW.format,
        'sha256'
    ), 'hex');
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_scarab_strategy_hash
    BEFORE INSERT OR UPDATE ON public.scarab_strategy
    FOR EACH ROW EXECUTE FUNCTION compute_strategy_hash();

-- Upsert helper for MCP tools
CREATE OR REPLACE FUNCTION upsert_scarab_strategy(
    p_service_id TEXT,
    p_optimizer TEXT DEFAULT 'adamw',
    p_hidden INTEGER DEFAULT 512,
    p_lr DOUBLE PRECISION DEFAULT 0.002,
    p_seed BIGINT DEFAULT 42,
    p_steps INTEGER DEFAULT 27000,
    p_ctx INTEGER DEFAULT 12,
    p_format TEXT DEFAULT 'fp32',
    p_status TEXT DEFAULT 'active',
    p_description TEXT DEFAULT NULL,
    p_canon_name TEXT DEFAULT NULL
) RETURNS void AS $$
BEGIN
    INSERT INTO public.scarab_strategy (
        service_id, optimizer, hidden, lr, seed, steps, ctx,
        format, status, description, canon_name, last_updated
    )
    VALUES (
        p_service_id, p_optimizer, p_hidden, p_lr, p_seed, p_steps, p_ctx,
        p_format, p_status, p_description, p_canon_name, NOW()
    )
    ON CONFLICT (service_id) DO UPDATE SET
        optimizer = EXCLUDED.optimizer,
        hidden = EXCLUDED.hidden,
        lr = EXCLUDED.lr,
        seed = EXCLUDED.seed,
        steps = EXCLUDED.steps,
        ctx = EXCLUDED.ctx,
        format = EXCLUDED.format,
        status = EXCLUDED.status,
        description = EXCLUDED.description,
        canon_name = EXCLUDED.canon_name,
        last_updated = NOW();
END;
$$ LANGUAGE plpgsql;

-- Heartbeat tracking table
CREATE TABLE IF NOT EXISTS public.scarab_heartbeat (
    service_id      TEXT PRIMARY KEY REFERENCES public.scarab_strategy(service_id) ON DELETE CASCADE,
    is_training     BOOLEAN NOT NULL DEFAULT false,
    current_hash    TEXT,
    last_heartbeat  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    trainer_pid     INTEGER,
    run_started_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_scarab_heartbeat_active
    ON public.scarab_heartbeat (is_training, last_heartbeat DESC);

-- Comments for documentation
COMMENT ON TABLE public.scarab_strategy IS 'Scarab strategy configuration - one row per scarab service';
COMMENT ON COLUMN public.scarab_strategy.service_id IS 'Unique identifier (typically RAILWAY_SERVICE_NAME)';
COMMENT ON COLUMN public.scarab_strategy.strategy_hash IS 'SHA-256 hash of config parameters for change detection';
COMMENT ON COLUMN public.scarab_strategy.status IS 'active=training, paused=not training, disabled=ignore';
COMMENT ON TABLE public.scarab_heartbeat IS 'Scarab liveness and training state tracking';

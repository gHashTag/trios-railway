-- Migration: audit_runs table for Zenodo Sentinel cron (skill_id a6d54f82, cron 1320bf84)
-- Sibling to gardener_runs from migrations/0001_railway_audit.sql
-- Tracked by: gHashTag/trios-railway#141
-- R5 rule: every green verdict MUST store evidence JSON with counts/sha-pins/response-codes

CREATE TABLE IF NOT EXISTS public.audit_runs (
  id           BIGSERIAL PRIMARY KEY,
  ts           TIMESTAMPTZ NOT NULL DEFAULT now(),
  probe        TEXT NOT NULL,        -- bib_dup | ref_label | includegraphics | citation_cff | zenodo_community | doi_head | tectonic
  verdict      TEXT NOT NULL,        -- green | yellow | red | skip
  anomalies    JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence     JSONB,
  duration_ms  INTEGER
);
CREATE INDEX IF NOT EXISTS idx_audit_runs_ts ON public.audit_runs (ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_runs_probe_ts ON public.audit_runs (probe, ts DESC);

-- After this lands, the Zenodo Sentinel cron auto-flushes the workspace buffer
-- (/home/user/workspace/zenodo_sentinel_buffer/*.json) via jsonb_array_elements on the first tick.

//! Idempotent Neon DDL for `tri railway audit migrate`.
//!
//! Tables live in the existing `public` schema next to the IGLA RACE
//! ledger so a single Neon role can read both halves.

use anyhow::{Context, Result};

/// Returns a slice of statements to run in order. All statements are
/// `CREATE … IF NOT EXISTS`, so re-running is a no-op.
#[must_use]
pub fn ddl_statements() -> &'static [&'static str] {
    DDL
}

/// Connect to Neon at `neon_url` and execute every DDL statement.
///
/// Returns the number of statements successfully executed.
/// Each statement is `CREATE IF NOT EXISTS` / `CREATE OR REPLACE`,
/// so re-running is always safe.
///
/// # Errors
///
/// Returns `Err` on connection failure or if any statement fails.
/// Never silently swallows errors (R5).
pub async fn run_migrate(neon_url: &str) -> Result<usize> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok(); // already installed is fine
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
    let (client, connection) = tokio_postgres::connect(neon_url, connector)
        .await
        .context("connect to Neon for DDL migration")?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("neon connection error: {e}");
        }
    });

    let stmts = ddl_statements();
    for (i, stmt) in stmts.iter().enumerate() {
        client
            .batch_execute(stmt)
            .await
            .with_context(|| format!("DDL statement {}/{} failed", i + 1, stmts.len()))?;
        tracing::debug!(i, total = stmts.len(), "DDL applied");
    }

    tracing::info!(total = stmts.len(), "all DDL statements applied");
    Ok(stmts.len())
}

const DDL: &[&str] = &[
    r"CREATE TABLE IF NOT EXISTS railway_projects (
        id              text PRIMARY KEY,
        name            text NOT NULL,
        workspace       text NOT NULL,
        default_env_id  text NOT NULL,
        observed_at     timestamptz NOT NULL DEFAULT now()
    )",
    r"CREATE TABLE IF NOT EXISTS railway_services (
        id              text PRIMARY KEY,
        project_id      text NOT NULL REFERENCES railway_projects(id),
        env_id          text NOT NULL,
        name            text NOT NULL,
        seed            integer,
        image           text,
        image_digest    text,
        last_deploy_id  text,
        last_status     text,
        created_at      timestamptz,
        observed_at     timestamptz NOT NULL DEFAULT now()
    )",
    r"CREATE TABLE IF NOT EXISTS railway_audit_runs (
        id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        agent           text NOT NULL,
        soul_name       text NOT NULL,
        phi_step        text NOT NULL CHECK (phi_step IN
                        ('CLAIM','NAME','SPEC','SEAL','GEN','TEST',
                         'VERDICT','EXPERIENCE','REPORT','COMMIT','PUSH')),
        started_at      timestamptz NOT NULL,
        finished_at     timestamptz,
        services_seen   integer,
        drift_events    integer,
        gate2_pass      boolean,
        target_bpb      double precision,
        artifact_url    text,
        experience_path text NOT NULL,
        exit_code       integer NOT NULL
    )",
    r"CREATE TABLE IF NOT EXISTS railway_audit_events (
        run_id          uuid REFERENCES railway_audit_runs(id) ON DELETE CASCADE,
        service_id      text,
        code            text NOT NULL,
        severity        text NOT NULL CHECK (severity IN ('warn','error','info')),
        detail          jsonb NOT NULL,
        triplet         text,
        PRIMARY KEY (run_id, service_id, code)
    )",
    r"CREATE OR REPLACE VIEW v_railway_drift_open AS
        SELECT e.code, e.severity, s.name AS service, e.triplet, r.started_at
        FROM railway_audit_events e
        JOIN railway_audit_runs   r ON r.id = e.run_id
        LEFT JOIN railway_services s ON s.id = e.service_id
        WHERE r.id = (SELECT id FROM railway_audit_runs ORDER BY started_at DESC LIMIT 1)",
    // AU-02: audit-event telemetry. Written by `event::audit_event()`.
    // NOTE: if the table already exists with a different schema (no `step`
    // column), CREATE IF NOT EXISTS is a no-op. The index below uses
    // DO NOTHING via a plpgsql block to avoid failures on divergent schemas.
    r"CREATE TABLE IF NOT EXISTS igla_race_trials (
        id          bigserial PRIMARY KEY,
        seed        integer   NOT NULL,
        bpb         double precision NOT NULL,
        step        integer   NOT NULL,
        image_sha   text      NOT NULL,
        recorded_at timestamptz NOT NULL DEFAULT now()
    )",
    r"DO $$ BEGIN
        IF EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = 'igla_race_trials' AND column_name = 'step'
        ) THEN
            CREATE INDEX IF NOT EXISTS igla_race_trials_seed_idx
                ON igla_race_trials (seed, step);
        END IF;
    END $$",
    // Gardener orchestrator run log. Written by tri-gardener neon.rs.
    r"CREATE TABLE IF NOT EXISTS gardener_runs (
        id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        ts            timestamptz NOT NULL DEFAULT now(),
        tick_t_minus  text         NOT NULL,
        action        text         NOT NULL,
        lane          text,
        seed          int,
        before_bpb    double precision,
        after_bpb     double precision,
        decision      jsonb        NOT NULL,
        audit_run_id  uuid REFERENCES railway_audit_runs(id) ON DELETE SET NULL
    )",
    r"CREATE INDEX IF NOT EXISTS gardener_runs_ts_idx
        ON gardener_runs (ts DESC)",
    // ===================================================================
    // ADR-0081 — Unified Experiment Loop (issue #81)
    //
    // Pull-based work-stealing queue. Gardener writes to `experiment_queue`
    // and reads `bpb_samples`. Seed Agent claims rows via
    // `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 1`, runs the trainer,
    // emits `bpb_samples` every 100 steps, makes the early-stop call at
    // step 1000.
    //
    // Status enum: pending | claimed | running | pruned | done | failed
    // R5-honest: status transitions are append-only audit (one row per
    // claim attempt) — never UPDATE-in-place silent moves.
    // ===================================================================
    r"CREATE TABLE IF NOT EXISTS experiment_queue (
        id              bigserial PRIMARY KEY,
        canon_name      text NOT NULL,
        config_json     jsonb NOT NULL,
        priority        integer NOT NULL DEFAULT 50
                        CHECK (priority BETWEEN 0 AND 100),
        seed            integer NOT NULL,
        steps_budget    integer NOT NULL
                        CHECK (steps_budget > 0),
        account         text NOT NULL
                        CHECK (account IN ('acc0','acc1','acc2','acc3','acc4','acc5')),
        status          text NOT NULL DEFAULT 'pending'
                        CHECK (status IN
                              ('pending','claimed','running','pruned','done','failed')),
        worker_id       uuid,
        prune_reason    text,
        final_bpb       double precision,
        final_step      integer,
        early_stop_bpb  double precision,
        created_at      timestamptz NOT NULL DEFAULT now(),
        claimed_at      timestamptz,
        started_at      timestamptz,
        finished_at     timestamptz,
        created_by      text NOT NULL DEFAULT 'gardener'
                        CHECK (created_by IN
                              ('gardener','human','auto-mirror','seed-agent'))
    )",
    // Pull-queue index — partial, only over rows that are actually
    // claimable. Keeps SKIP LOCKED scans cheap as the table grows.
    // DESC matches claim SQL `ORDER BY priority DESC` for index-only scan.
    r"CREATE INDEX IF NOT EXISTS experiment_queue_pull_idx
        ON experiment_queue (priority DESC, created_at ASC)
        WHERE status = 'pending'",
    // Lookup by canon for gardener strategy ticks.
    r"CREATE INDEX IF NOT EXISTS experiment_queue_canon_idx
        ON experiment_queue (canon_name, seed)",
    // Stale-claim recovery: gardener resets rows whose claimed_at is
    // older than 5 minutes back to 'pending'. Index keeps that scan O(log n).
    r"CREATE INDEX IF NOT EXISTS experiment_queue_stale_claim_idx
        ON experiment_queue (claimed_at)
        WHERE status = 'claimed'",
    // BPB telemetry — one row per (canon, seed, step). Already referenced
    // by `bin/tri-gardener/src/bpb_source.rs`; this DDL is the canonical
    // create. Issue #62 noted Pipedream silently rolls DDL back; apply
    // via psql for reliable schema.
    r"CREATE TABLE IF NOT EXISTS bpb_samples (
        id          bigserial PRIMARY KEY,
        canon_name  text NOT NULL,
        seed        integer NOT NULL,
        step        integer NOT NULL CHECK (step >= 0),
        bpb         double precision NOT NULL,
        val_bpb_ema double precision,
        ts          timestamptz NOT NULL DEFAULT now(),
        UNIQUE (canon_name, seed, step)
    )",
    r"CREATE INDEX IF NOT EXISTS bpb_samples_canon_seed_step_idx
        ON bpb_samples (canon_name, seed, step DESC)",
    r"CREATE INDEX IF NOT EXISTS bpb_samples_recent_idx
        ON bpb_samples (ts DESC)",
    // Worker registry. Heartbeat updated by Seed Agent at every Neon
    // poll. Stale workers (no heartbeat > 2 minutes) are evicted by the
    // gardener and their claimed experiments are returned to 'pending'.
    r"CREATE TABLE IF NOT EXISTS workers (
        id              uuid PRIMARY KEY,
        railway_acc     text NOT NULL
                        CHECK (railway_acc IN ('acc0','acc1','acc2','acc3','acc4','acc5')),
        railway_svc_id  text NOT NULL,
        railway_svc_name text NOT NULL,
        last_heartbeat  timestamptz NOT NULL DEFAULT now(),
        current_exp_id  bigint REFERENCES experiment_queue(id) ON DELETE SET NULL,
        registered_at   timestamptz NOT NULL DEFAULT now()
    )",
    r"CREATE INDEX IF NOT EXISTS workers_heartbeat_idx
        ON workers (last_heartbeat DESC)",
    // Audit trail of strategic decisions made by the gardener. Append-only.
    r"CREATE TABLE IF NOT EXISTS gardener_decisions (
        id              bigserial PRIMARY KEY,
        ts              timestamptz NOT NULL DEFAULT now(),
        action          text NOT NULL
                        CHECK (action IN
                              ('enqueue','prune','priority_boost',
                               'spawn_mirror','reset_stale_claim','noop')),
        affected_exp_ids bigint[] NOT NULL DEFAULT '{}',
        reason          text NOT NULL,
        snapshot        jsonb
    )",
    r"CREATE INDEX IF NOT EXISTS gardener_decisions_ts_idx
        ON gardener_decisions (ts DESC)",
    // Live-leaderboard view: best (lowest) BPB per canon+seed across all
    // samples, joined with experiment status. Used by gardener strategy
    // and `mcp.fleet.snapshot`.
    r"CREATE OR REPLACE VIEW v_leaderboard AS
        SELECT
            q.canon_name,
            q.seed,
            q.account,
            q.status,
            q.priority,
            COALESCE(b.best_bpb, q.final_bpb) AS best_bpb,
            b.last_step,
            b.last_ts,
            q.created_at,
            q.finished_at
        FROM experiment_queue q
        LEFT JOIN (
            SELECT canon_name, seed,
                   MIN(bpb) AS best_bpb,
                   MAX(step) AS last_step,
                   MAX(ts)   AS last_ts
            FROM bpb_samples
            GROUP BY canon_name, seed
        ) b ON b.canon_name = q.canon_name AND b.seed = q.seed",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddl_is_nonempty_and_idempotent_friendly() {
        let v = ddl_statements();
        assert!(!v.is_empty());
        for stmt in v {
            // Either CREATE TABLE IF NOT EXISTS or CREATE OR REPLACE VIEW.
            assert!(
                stmt.contains("IF NOT EXISTS") || stmt.contains("OR REPLACE"),
                "non-idempotent DDL: {stmt}"
            );
        }
    }

    /// All canonical tables must be present in `ddl_statements()`.
    /// If anyone removes one the test fails loudly (R5).
    #[test]
    fn all_canonical_tables_are_present() {
        let blob: String = ddl_statements().join("\n");
        for needle in [
            "CREATE TABLE IF NOT EXISTS railway_projects",
            "CREATE TABLE IF NOT EXISTS railway_services",
            "CREATE TABLE IF NOT EXISTS railway_audit_runs",
            "CREATE TABLE IF NOT EXISTS railway_audit_events",
            "CREATE TABLE IF NOT EXISTS igla_race_trials",
            "CREATE TABLE IF NOT EXISTS gardener_runs",
            "CREATE TABLE IF NOT EXISTS experiment_queue",
            "CREATE TABLE IF NOT EXISTS bpb_samples",
            "CREATE TABLE IF NOT EXISTS workers",
            "CREATE TABLE IF NOT EXISTS gardener_decisions",
            "CREATE OR REPLACE VIEW v_leaderboard",
        ] {
            assert!(blob.contains(needle), "missing DDL: {needle}");
        }
    }

    /// Pull-queue index must be partial on `status='pending'` so the
    /// SKIP LOCKED scan stays cheap as the table grows.
    #[test]
    fn experiment_queue_pull_index_is_partial() {
        let blob: String = ddl_statements().join("\n");
        assert!(blob.contains("experiment_queue_pull_idx"));
        // Find the line and assert it carries the WHERE clause.
        let idx_block = blob
            .split("experiment_queue_pull_idx")
            .nth(1)
            .expect("pull idx fragment");
        assert!(
            idx_block.contains("WHERE status = 'pending'"),
            "pull idx must be partial on status=pending"
        );
    }

    /// Status enum is the single source of truth for legal
    /// `experiment_queue.status` values. Any drift between this list
    /// and the CHECK constraint in the DDL trips the test.
    #[test]
    fn experiment_queue_status_enum_is_locked() {
        let blob: String = ddl_statements().join("\n");
        for s in ["pending", "claimed", "running", "pruned", "done", "failed"] {
            assert!(
                blob.contains(&format!("'{s}'")),
                "experiment_queue status `{s}` missing from DDL"
            );
        }
    }

    /// Account whitelist matches `RailwayMultiClient::AccountId::all()`.
    #[test]
    fn experiment_queue_account_enum_matches_multiclient() {
        let blob: String = ddl_statements().join("\n");
        for a in ["acc0", "acc1", "acc2", "acc3", "acc4", "acc5"] {
            assert!(
                blob.contains(&format!("'{a}'")),
                "account `{a}` missing from DDL CHECK"
            );
        }
    }

    /// `bpb_samples` must enforce step uniqueness so the trainer's
    /// `INSERT ... ON CONFLICT DO NOTHING` path is well-defined.
    #[test]
    fn bpb_samples_has_canon_seed_step_unique() {
        let blob: String = ddl_statements().join("\n");
        let bpb_block = blob
            .split("CREATE TABLE IF NOT EXISTS bpb_samples")
            .nth(1)
            .expect("bpb_samples DDL fragment");
        assert!(
            bpb_block.contains("UNIQUE (canon_name, seed, step)"),
            "bpb_samples must declare (canon_name, seed, step) UNIQUE"
        );
    }

    /// `gardener_decisions.action` enum is the single source of truth
    /// for orchestrator audit-log values.
    #[test]
    fn gardener_decisions_action_enum_is_locked() {
        let blob: String = ddl_statements().join("\n");
        for a in [
            "enqueue",
            "prune",
            "priority_boost",
            "spawn_mirror",
            "reset_stale_claim",
            "noop",
        ] {
            assert!(
                blob.contains(&format!("'{a}'")),
                "gardener_decisions action `{a}` missing from DDL"
            );
        }
    }

    /// `igla_race_trials` must have a seed+step index for audit lookups.
    #[test]
    fn igla_race_trials_has_seed_step_index() {
        let blob: String = ddl_statements().join("\n");
        assert!(
            blob.contains("igla_race_trials_seed_idx"),
            "igla_race_trials missing seed+step index"
        );
        assert!(
            blob.contains("igla_race_trials"),
            "igla_race_trials table DDL missing"
        );
    }

    /// `gardener_runs` must have a ts index for the gardener dashboard.
    #[test]
    fn gardener_runs_has_ts_index() {
        let blob: String = ddl_statements().join("\n");
        assert!(
            blob.contains("gardener_runs_ts_idx"),
            "gardener_runs missing ts index"
        );
    }
}

/// Text-level invariants for the standalone SQL migration files in
/// `migrations/` that are applied by the `apply-migrations.yml` workflow
/// rather than the `tri railway audit migrate` Rust path. These checks
/// guard the L-SS4/L-SS5 (Sovereign Scarab v4) heartbeat-TTL janitor
/// contract — see gHashTag/trios-railway#212 and the G-SS-10 contract.
#[cfg(test)]
mod sql_file_invariants {
    /// `migrations/0013_heartbeat_janitor.sql` carries the advisory-lock
    /// janitor for L-SS5. The file is applied verbatim by the
    /// `apply-migrations.yml` workflow, so any drift away from the
    /// G-SS-10 contract must trip CI before it reaches prod.
    const M_0013: &str = include_str!("../../../migrations/0013_heartbeat_janitor.sql");

    /// L-SS4 dependency: `scarab_dead` view defines what "stale" means.
    const M_0012: &str = include_str!("../../../migrations/0012_scarab_dead_heartbeat.sql");

    /// The janitor function must exist with the documented signature.
    #[test]
    fn janitor_function_exists() {
        assert!(
            M_0013.contains("FUNCTION ssot.janitor_release_stale()"),
            "0013 must define ssot.janitor_release_stale()"
        );
        assert!(
            M_0013.contains("RETURNS integer"),
            "janitor must return an integer (released row count)"
        );
    }

    /// G-SS-10: the advisory-lock key is exactly `0xDEADBEEF`.
    /// Both the literal hex and its decimal value (3735928559) appear
    /// in the file (literal in the function, decimal in the COMMENT) —
    /// guard against silent drift of either.
    #[test]
    fn advisory_lock_key_is_deadbeef() {
        assert!(
            M_0013.contains("x'DEADBEEF'::bigint"),
            "advisory lock key must be the hex literal 0xDEADBEEF"
        );
        assert!(
            M_0013.contains("3735928559"),
            "decimal form 3735928559 must appear (documents the key in COMMENT)"
        );
        // 0xDEADBEEF must equal 3735928559 — guard against typos.
        assert_eq!(0xDEAD_BEEF_u64, 3_735_928_559_u64);
    }

    /// G-SS-10: acquisition MUST be non-blocking. `pg_advisory_lock`
    /// (blocking) would let the janitor pile up sessions if a previous
    /// run hung. Only `pg_try_advisory_lock` is allowed.
    #[test]
    fn advisory_lock_acquisition_is_non_blocking() {
        assert!(
            M_0013.contains("pg_try_advisory_lock"),
            "must use pg_try_advisory_lock (non-blocking)"
        );
        // No blocking variant — search for the exact call (not the `_try_` one).
        // A simple substring check would false-match `pg_try_advisory_lock`, so
        // strip those first.
        let stripped = M_0013.replace("pg_try_advisory_lock", "");
        assert!(
            !stripped.contains("pg_advisory_lock("),
            "blocking pg_advisory_lock() must not be used for janitor acquisition"
        );
    }

    /// G-SS-10: the lock must be released in BOTH the normal path and
    /// the EXCEPTION handler, otherwise a failing janitor leaks a
    /// session-level lock and bricks the next run.
    #[test]
    fn advisory_lock_released_in_both_paths() {
        // EXCEPTION block exists and releases the lock.
        assert!(
            M_0013.contains("EXCEPTION WHEN OTHERS THEN"),
            "must have EXCEPTION WHEN OTHERS handler"
        );
        // Two unlock calls: one normal, one in the exception handler.
        let unlock_count = M_0013.matches("pg_advisory_unlock").count();
        assert!(
            unlock_count >= 2,
            "must call pg_advisory_unlock at least twice (normal + exception), got {unlock_count}"
        );
    }

    /// L-SS5 spec: stale = heartbeat older than 600 seconds.
    /// The threshold lives in this one place — drift would silently
    /// change janitor semantics.
    #[test]
    fn stale_threshold_is_600_seconds() {
        assert!(
            M_0013.contains("age_seconds > 600"),
            "janitor must release scarabs with age_seconds > 600 (L-SS5)"
        );
    }

    /// The janitor mutates `ssot.scarab_strategy` by setting
    /// `status = 'released'` on dead rows. Anything else would silently
    /// change SSOT semantics.
    #[test]
    fn janitor_transitions_to_released_status() {
        assert!(
            M_0013.contains("UPDATE ssot.scarab_strategy"),
            "janitor must update ssot.scarab_strategy"
        );
        assert!(
            M_0013.contains("status = 'released'"),
            "janitor must transition rows to status='released'"
        );
        assert!(
            M_0013.contains("FROM   ssot.scarab_dead") || M_0013.contains("FROM ssot.scarab_dead"),
            "janitor must source candidates from the ssot.scarab_dead view (L-SS4)"
        );
    }

    /// SECURITY DEFINER + a COMMENT linking back to issue #212 are part
    /// of the L-SS5 acceptance checklist.
    #[test]
    fn function_is_security_definer_with_issue_comment() {
        assert!(
            M_0013.contains("SECURITY DEFINER"),
            "janitor function must be SECURITY DEFINER"
        );
        assert!(
            M_0013.contains("#212"),
            "function COMMENT must reference issue #212"
        );
    }

    /// `ssot.janitor_status` view exposes lock-holder PID for fleet
    /// observability (L-SS5 acceptance criterion).
    #[test]
    fn janitor_status_view_exposes_lock_holder() {
        assert!(
            M_0013.contains("VIEW ssot.janitor_status"),
            "0013 must define ssot.janitor_status view"
        );
        assert!(
            M_0013.contains("lock_holder_pid"),
            "view must surface lock_holder_pid"
        );
        assert!(
            M_0013.contains("released_last_hour"),
            "view must surface released_last_hour"
        );
        assert!(
            M_0013.contains("pending_release_count"),
            "view must surface pending_release_count"
        );
    }

    /// L-SS4 (#211, closes #193): `scarab_dead` must be heartbeat-based,
    /// not `bpb_samples` push-path based. 0013 layers on top of this; if
    /// 0012 ever regresses to `bpb_samples`, the janitor will release the
    /// wrong rows.
    #[test]
    fn scarab_dead_is_heartbeat_based() {
        assert!(
            M_0012.contains("ssot.scarab_heartbeat"),
            "0012 scarab_dead must derive staleness from scarab_heartbeat"
        );
        assert!(
            M_0012.contains("INTERVAL '120 seconds'"),
            "scarab_dead threshold is 120s (per L-SS4 spec)"
        );
    }

    /// Migrations are applied in lexical order (see
    /// `.github/workflows/apply-migrations.yml`). 0013 depends on the
    /// `ssot.scarab_dead` view created by 0012; if anyone renames or
    /// reorders the files this guard trips.
    #[test]
    fn migration_files_are_ordered_lexically() {
        // include_str!() at module top resolves at compile time, so the
        // mere fact this module compiles proves both files exist at
        // their canonical paths and are readable. This test additionally
        // asserts the ordering invariant the janitor depends on.
        let mut entries: Vec<_> =
            std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations"))
                .expect("migrations/ directory must exist")
                .filter_map(std::result::Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| {
                    std::path::Path::new(n)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
                })
                .collect();
        entries.sort();
        let idx_0012 = entries
            .iter()
            .position(|n| n.starts_with("0012_"))
            .expect("0012 migration must exist");
        let idx_0013 = entries
            .iter()
            .position(|n| n.starts_with("0013_"))
            .expect("0013 migration must exist");
        assert!(
            idx_0012 < idx_0013,
            "0012 (scarab_dead) must apply before 0013 (janitor)"
        );
    }
}

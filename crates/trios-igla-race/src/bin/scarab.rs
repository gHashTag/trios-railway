//! Sovereign Scarab v4 pull-loop runtime (ADR-0042).
//!
//! One deploy, forever live. Each scarab:
//!   1. Resolves its own stable `service_id` from `RAILWAY_SERVICE_ID`.
//!   2. Connects to the Trinity SSOT via `DATABASE_URL`
//!      (legacy `RAILWAY_POSTGRES_URL` / `NEON_DATABASE_URL` accepted).
//!   3. Polls `ssot.scarab_strategy WHERE service_id = $me` every N seconds.
//!   4. Upserts `ssot.scarab_heartbeat` on startup and every tick — even
//!      before any BPB samples exist (this is the runtime liveness signal
//!      Queen Hive watches; it must beat with or without a trainer child).
//!   5. On `generation` bump or `status` change: graceful in-process
//!      restart of the trainer subprocess. No Railway API, no
//!      `variableUpsert`, no redeploy.
//!   6. Trainer child writes BPB samples to `ssot.bpb_samples` via the
//!      scarab parent forwarding parsed `step=N val_bpb=F` lines (mirrors
//!      the seed-agent telemetry contract).
//!
//! ENV:
//!   DATABASE_URL              — required (preferred name in ADR-0042 §2).
//!     RAILWAY_POSTGRES_URL    — legacy, accepted.
//!     NEON_DATABASE_URL       — legacy, accepted.
//!   RAILWAY_SERVICE_ID        — required (Railway sets this automatically).
//!     SCARAB_SERVICE_ID       — manual override for local runs / tests.
//!   HEARTBEAT_INTERVAL_S      — heartbeat cadence, default 30 s.
//!   POLL_INTERVAL_MS          — strategy poll cadence, default 10000.
//!   TRAINER_BIN               — trainer binary path, default `/usr/local/bin/trios-train`.
//!   SCARAB_ACCOUNT            — cosmetic log tag (no routing effect).
//!
//! Anchor: phi^2 + phi^-2 = 3.

use std::env;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_postgres::Client;

/// SQL: read the strategy row for this scarab. Returns at most one row.
/// Pulls the trainer_bin / w_jepa / w_nca axes added in migration 0010,
/// falling back to `'trios-train'` if those columns are absent (older DB).
const STRATEGY_SELECT_SQL: &str = "\
    SELECT optimizer, format, hidden, lr::float8, seed, steps, status, \
           generation, \
           COALESCE(trainer_bin, 'trios-train') AS trainer_bin, \
           COALESCE(w_jepa, 0)::float8 AS w_jepa, \
           COALESCE(w_nca, 0)::float8 AS w_nca \
    FROM ssot.scarab_strategy \
    WHERE service_id = $1";

/// SQL: heartbeat upsert. Always emitted, even when no trainer is running.
/// `applied_version` records which `generation` the scarab has converged on.
const HEARTBEAT_UPSERT_SQL: &str = "\
    INSERT INTO ssot.scarab_heartbeat \
        (service_id, last_seen, current_gen, current_step, current_bpb, \
         pid, started_at, applied_version) \
    VALUES ($1, now(), $2, $3, $4, $5, $6, $2) \
    ON CONFLICT (service_id) DO UPDATE SET \
        last_seen       = now(), \
        current_gen     = EXCLUDED.current_gen, \
        current_step    = EXCLUDED.current_step, \
        current_bpb     = EXCLUDED.current_bpb, \
        pid             = EXCLUDED.pid, \
        started_at      = EXCLUDED.started_at, \
        applied_version = EXCLUDED.applied_version";

/// SQL: BPB sample insert. Schema per the IGLA RACE blocker context:
/// (id, canon_name, format, algo, hidden, seed, step, bpb, sha, run_id, ts).
/// We populate the columns we can derive from the strategy row; `sha`
/// and `run_id` are left NULL (acceptable per the ledger schema — they
/// are diagnostic metadata, not required for the trajectory).
const BPB_INSERT_SQL: &str = "\
    INSERT INTO ssot.bpb_samples \
        (canon_name, format, algo, hidden, seed, step, bpb, ts) \
    VALUES ($1, $2, $3, $4, $5, $6, $7, now()) \
    ON CONFLICT DO NOTHING";

#[derive(Debug, Clone, PartialEq)]
pub struct Strategy {
    pub optimizer: String,
    pub format: String,
    pub hidden: i32,
    pub lr: f64,
    pub seed: i32,
    pub steps: i32,
    pub status: String,
    pub generation: i64,
    pub trainer_bin: String,
    pub w_jepa: f64,
    pub w_nca: f64,
}

impl Strategy {
    /// Two strategy snapshots are "config-equivalent" if every trainable
    /// knob matches. We re-check `generation` separately because a Queen
    /// bump always increments it, but operators can also poke
    /// generation manually without changing any field — both paths
    /// should trigger a respawn.
    #[must_use]
    pub fn same_config(&self, other: &Self) -> bool {
        self.optimizer == other.optimizer
            && self.format == other.format
            && self.hidden == other.hidden
            && (self.lr - other.lr).abs() < f64::EPSILON
            && self.seed == other.seed
            && self.steps == other.steps
            && self.trainer_bin == other.trainer_bin
            && (self.w_jepa - other.w_jepa).abs() < f64::EPSILON
            && (self.w_nca - other.w_nca).abs() < f64::EPSILON
    }

    /// Should the scarab (re)launch the trainer for this strategy?
    /// Returns `true` when:
    ///   * `status == 'active'` and we have no prior generation,
    ///   * `status == 'active'` and the generation advanced,
    ///   * `status == 'active'` and any config field changed under us.
    /// Returns `false` for `'paused'` / `'stop'` — the trainer should be
    /// stopped (handled by the caller) and the scarab keeps heart-beating.
    #[must_use]
    pub fn needs_restart_from(&self, last_applied: Option<&Self>) -> bool {
        if self.status != "active" {
            return false;
        }
        match last_applied {
            None => true,
            Some(prev) => self.generation != prev.generation || !self.same_config(prev),
        }
    }

    /// Canonical lane name (matches `mass-revive-strategy.yml`):
    /// `IGLA-STRATEGY-{format}-h{hidden}-LR{lr}-rng{seed}-{optimizer}`.
    #[must_use]
    pub fn canon_name(&self) -> String {
        format!(
            "IGLA-STRATEGY-{}-h{}-LR{}-rng{}-{}",
            self.format, self.hidden, self.lr, self.seed, self.optimizer
        )
    }
}

/// Resolve `service_id` exactly the way ADR-0042 §2 prescribes:
/// Railway injects `RAILWAY_SERVICE_ID` at runtime; manual deploys (and
/// local tests) may set `SCARAB_SERVICE_ID` as an override. We refuse
/// to start with no identity rather than silently picking a wrong row.
pub fn resolve_service_id() -> Result<String> {
    if let Ok(v) = env::var("SCARAB_SERVICE_ID") {
        if !v.is_empty() {
            return Ok(v);
        }
    }
    if let Ok(v) = env::var("RAILWAY_SERVICE_ID") {
        if !v.is_empty() {
            return Ok(v);
        }
    }
    anyhow::bail!(
        "neither RAILWAY_SERVICE_ID nor SCARAB_SERVICE_ID is set — cannot \
         resolve scarab identity. Refusing to start (ADR-0042 §2)."
    )
}

/// Resolve the Trinity SSOT URL. ADR-0042 prefers `DATABASE_URL`; we
/// keep the two legacy names accepted so existing deploys keep working.
pub fn resolve_db_url() -> Result<String> {
    for key in ["DATABASE_URL", "RAILWAY_POSTGRES_URL", "NEON_DATABASE_URL"] {
        if let Ok(v) = env::var(key) {
            if !v.is_empty() {
                return Ok(v);
            }
        }
    }
    anyhow::bail!(
        "none of DATABASE_URL / RAILWAY_POSTGRES_URL / NEON_DATABASE_URL \
         are set — cannot connect to Trinity SSOT. Refusing to start."
    )
}

async fn connect(db_url: &str) -> Result<Client> {
    let mut builder = SslConnector::builder(SslMethod::tls())?;
    builder.set_verify(SslVerifyMode::NONE);
    let connector = MakeTlsConnector::new(builder.build());
    let (client, conn) = tokio_postgres::connect(db_url, connector)
        .await
        .with_context(|| "connect to Trinity SSOT")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[scarab] postgres connection lost: {e}");
        }
    });
    Ok(client)
}

pub async fn fetch_strategy(client: &Client, service_id: &str) -> Result<Option<Strategy>> {
    let row = client
        .query_opt(STRATEGY_SELECT_SQL, &[&service_id])
        .await
        .with_context(|| "SELECT ssot.scarab_strategy")?;
    let Some(row) = row else { return Ok(None) };
    Ok(Some(Strategy {
        optimizer: row.get(0),
        format: row.get(1),
        hidden: row.get(2),
        lr: row.get(3),
        seed: row.get(4),
        steps: row.get(5),
        status: row.get(6),
        generation: row.get(7),
        trainer_bin: row.get(8),
        w_jepa: row.get(9),
        w_nca: row.get(10),
    }))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LiveStats {
    pub step: Option<i32>,
    pub bpb: Option<f64>,
}

pub async fn upsert_heartbeat(
    client: &Client,
    service_id: &str,
    applied_gen: i64,
    trainer_pid: Option<i32>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    stats: LiveStats,
) -> Result<()> {
    client
        .execute(
            HEARTBEAT_UPSERT_SQL,
            &[
                &service_id,
                &applied_gen,
                &stats.step,
                &stats.bpb,
                &trainer_pid,
                &started_at,
            ],
        )
        .await
        .with_context(|| "UPSERT ssot.scarab_heartbeat")?;
    Ok(())
}

pub async fn push_bpb_sample(
    client: &Client,
    strategy: &Strategy,
    step: i32,
    bpb: f64,
) -> Result<()> {
    let canon = strategy.canon_name();
    client
        .execute(
            BPB_INSERT_SQL,
            &[
                &canon,
                &strategy.format,
                &strategy.optimizer,
                &strategy.hidden,
                &strategy.seed,
                &step,
                &bpb,
            ],
        )
        .await
        .with_context(|| "INSERT ssot.bpb_samples")?;
    Ok(())
}

fn parse_duration_ms(key: &str, default_ms: u64) -> Duration {
    let raw = env::var(key).ok();
    let ms = raw
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
}

fn parse_duration_s(key: &str, default_s: u64) -> Duration {
    let raw = env::var(key).ok();
    let s = raw
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default_s);
    Duration::from_secs(s)
}

/// Parse one `trios-train` stdout line into a `(step, bpb)` pair if it
/// matches `step=N val_bpb=F`. Returns None for any other line.
#[must_use]
pub fn parse_trainer_line(line: &str) -> Option<(i32, f64)> {
    let step_pos = line.find("step=")?;
    let after_step = &line[step_pos + 5..];
    let step_end = after_step.find(char::is_whitespace)?;
    let step: i32 = after_step[..step_end].trim().parse().ok()?;

    let bpb_pos = line.find("val_bpb=")?;
    let after_bpb = &line[bpb_pos + 8..];
    let bpb_end = after_bpb
        .find(char::is_whitespace)
        .unwrap_or(after_bpb.len());
    let bpb: f64 = after_bpb[..bpb_end].parse().ok()?;
    Some((step, bpb))
}

/// Spawn the trainer subprocess. CLI matches the `trios-train`
/// flag-set verified by seed-agent's `ExternalTrainer::spawn` (see
/// `bin/seed-agent/src/trainer.rs`): `--seed`, `--steps`, `--hidden`,
/// `--lr`, `--optimizer`. We do NOT pass `--ctx` / `--format` because
/// the trainer rejected those flags in production (commit history in
/// `trainer.rs`).
async fn spawn_trainer(strategy: &Strategy, trainer_bin: &str) -> Result<Child> {
    let mut cmd = Command::new(trainer_bin);
    cmd.arg("--seed")
        .arg(strategy.seed.to_string())
        .arg("--steps")
        .arg(strategy.steps.to_string())
        .arg("--hidden")
        .arg(strategy.hidden.to_string())
        .arg("--lr")
        .arg(format!("{:.6}", strategy.lr))
        .arg("--optimizer")
        .arg(&strategy.optimizer);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    let child = cmd
        .spawn()
        .with_context(|| format!("spawn trainer {trainer_bin:?}"))?;
    Ok(child)
}

/// Background task that streams trainer stdout, parses BPB lines, and
/// pushes them to `ssot.bpb_samples` + updates the shared `LiveStats`
/// the main loop reports through heartbeat.
fn spawn_stdout_pump(
    child_stdout: tokio::process::ChildStdout,
    client: Arc<Client>,
    strategy: Strategy,
    stats: Arc<Mutex<LiveStats>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(child_stdout).lines();
        loop {
            match reader.next_line().await {
                Ok(Some(line)) => {
                    println!("[trainer] {line}");
                    if let Some((step, bpb)) = parse_trainer_line(&line) {
                        {
                            let mut s = stats.lock().await;
                            s.step = Some(step);
                            s.bpb = Some(bpb);
                        }
                        if let Err(e) = push_bpb_sample(&client, &strategy, step, bpb).await {
                            eprintln!("[scarab] bpb sample push failed: {e:#}");
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("[scarab] trainer stdout read error: {e}");
                    break;
                }
            }
        }
    })
}

/// Stop the trainer gracefully (best-effort SIGKILL; `kill_on_drop`
/// also reaps on Drop). Awaits the wait status to avoid zombies.
async fn stop_trainer(child: &mut Child) {
    let _ = child.start_kill();
    let _ = child.wait().await;
}

#[tokio::main]
async fn main() -> Result<()> {
    let service_id = resolve_service_id()?;
    let db_url = resolve_db_url()?;
    let label = env::var("SCARAB_ACCOUNT").unwrap_or_else(|_| "scarab".into());
    let trainer_bin =
        env::var("TRAINER_BIN").unwrap_or_else(|_| "/usr/local/bin/trios-train".into());
    let poll_interval = parse_duration_ms("POLL_INTERVAL_MS", 10_000);
    let heartbeat_interval = parse_duration_s("HEARTBEAT_INTERVAL_S", 30);

    println!(
        "[scarab] boot service_id={service_id} acc={label} \
         poll={}ms hb={}s trainer={trainer_bin}",
        poll_interval.as_millis(),
        heartbeat_interval.as_secs(),
    );

    let client = Arc::new(connect(&db_url).await?);

    // Startup heartbeat: declare presence before fetching strategy so
    // operators can see the scarab is alive even if its strategy row is
    // missing or paused.
    upsert_heartbeat(&client, &service_id, 0, None, None, LiveStats::default()).await?;

    let mut applied: Option<Strategy> = None;
    let mut trainer: Option<Child> = None;
    let mut pump: Option<tokio::task::JoinHandle<()>> = None;
    let mut started_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_heartbeat = std::time::Instant::now()
        .checked_sub(heartbeat_interval)
        .unwrap_or_else(std::time::Instant::now);
    let stats = Arc::new(Mutex::new(LiveStats::default()));

    loop {
        match fetch_strategy(&client, &service_id).await {
            Ok(Some(strategy)) => {
                if strategy.status != "active" {
                    if let Some(child) = trainer.as_mut() {
                        println!("[scarab] status={} -> stopping trainer", strategy.status);
                        stop_trainer(child).await;
                        trainer = None;
                        started_at = None;
                        if let Some(h) = pump.take() {
                            h.abort();
                        }
                        *stats.lock().await = LiveStats::default();
                    }
                } else if strategy.needs_restart_from(applied.as_ref()) {
                    if let Some(child) = trainer.as_mut() {
                        println!(
                            "[scarab] generation {} -> {} : restarting trainer",
                            applied.as_ref().map_or(0, |a| a.generation),
                            strategy.generation,
                        );
                        stop_trainer(child).await;
                        if let Some(h) = pump.take() {
                            h.abort();
                        }
                    } else {
                        println!(
                            "[scarab] launching trainer at generation {}",
                            strategy.generation
                        );
                    }
                    *stats.lock().await = LiveStats::default();
                    match spawn_trainer(&strategy, &trainer_bin).await {
                        Ok(mut child) => {
                            started_at = Some(chrono::Utc::now());
                            let stdout = child
                                .stdout
                                .take()
                                .ok_or_else(|| anyhow::anyhow!("trainer stdout pipe missing"))?;
                            pump = Some(spawn_stdout_pump(
                                stdout,
                                client.clone(),
                                strategy.clone(),
                                stats.clone(),
                            ));
                            trainer = Some(child);
                        }
                        Err(e) => {
                            eprintln!(
                                "[scarab] trainer spawn failed (will retry next tick): {e:#}"
                            );
                            trainer = None;
                            started_at = None;
                        }
                    }
                }
                applied = Some(strategy);
            }
            Ok(None) => {
                // No row for this service_id yet. Keep heart-beating so
                // operators can see the scarab is alive and waiting.
                eprintln!(
                    "[scarab] no ssot.scarab_strategy row for service_id={service_id} \
                     (waiting for Queen Hive to insert)"
                );
            }
            Err(e) => {
                eprintln!("[scarab] strategy poll error: {e:#}");
            }
        }

        // Reap exited trainer so a fresh strategy bump can respawn it.
        if let Some(child) = trainer.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("[scarab] trainer exited: {status}");
                    trainer = None;
                    started_at = None;
                    if let Some(h) = pump.take() {
                        h.abort();
                    }
                    *stats.lock().await = LiveStats::default();
                }
                Ok(None) => { /* still running */ }
                Err(e) => eprintln!("[scarab] try_wait failed: {e}"),
            }
        }

        // Heartbeat at the configured cadence (and at least once per tick
        // for fresh boots where last_heartbeat was clamped to now()).
        if last_heartbeat.elapsed() >= heartbeat_interval {
            let applied_gen = applied.as_ref().map_or(0, |s| s.generation);
            let pid = trainer
                .as_ref()
                .and_then(|c| c.id().map(|p| i32::try_from(p).unwrap_or(i32::MAX)));
            let snap = *stats.lock().await;
            if let Err(e) =
                upsert_heartbeat(&client, &service_id, applied_gen, pid, started_at, snap).await
            {
                eprintln!("[scarab] heartbeat upsert failed: {e:#}");
            } else {
                last_heartbeat = std::time::Instant::now();
            }
        }

        sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(generation: i64, status: &str, hidden: i32, lr: f64) -> Strategy {
        Strategy {
            optimizer: "adamw".into(),
            format: "fp32".into(),
            hidden,
            lr,
            seed: 1597,
            steps: 30_000,
            status: status.into(),
            generation,
            trainer_bin: "trios-train".into(),
            w_jepa: 0.0,
            w_nca: 0.0,
        }
    }

    #[test]
    fn needs_restart_when_no_prior_strategy() {
        let cur = s(1, "active", 384, 0.001);
        assert!(cur.needs_restart_from(None));
    }

    #[test]
    fn no_restart_when_paused() {
        let cur = s(1, "paused", 384, 0.001);
        assert!(!cur.needs_restart_from(None));
    }

    #[test]
    fn no_restart_when_stop() {
        let cur = s(7, "stop", 384, 0.001);
        let prev = s(7, "active", 384, 0.001);
        assert!(!cur.needs_restart_from(Some(&prev)));
    }

    #[test]
    fn restart_on_generation_bump() {
        let prev = s(1, "active", 384, 0.001);
        let cur = s(2, "active", 384, 0.001);
        assert!(cur.needs_restart_from(Some(&prev)));
    }

    #[test]
    fn restart_on_config_change_without_generation_bump() {
        // Defensive guard: even if some path forgets to increment
        // generation, a config drift must still re-launch.
        let prev = s(1, "active", 384, 0.001);
        let cur = s(1, "active", 512, 0.001);
        assert!(cur.needs_restart_from(Some(&prev)));
        let cur_lr = s(1, "active", 384, 0.002);
        assert!(cur_lr.needs_restart_from(Some(&prev)));
    }

    #[test]
    fn no_restart_when_unchanged() {
        let prev = s(5, "active", 384, 0.001);
        let cur = s(5, "active", 384, 0.001);
        assert!(!cur.needs_restart_from(Some(&prev)));
    }

    #[test]
    fn canon_name_matches_mass_revive_pattern() {
        let strat = s(1, "active", 384, 0.0001);
        let mut strat = strat;
        strat.seed = 1597;
        strat.format = "gf64".into();
        strat.hidden = 512;
        strat.optimizer = "adamw".into();
        // Matches `IGLA-STRATEGY-{format}-h{hidden}-LR{lr}-rng{seed}-{optimizer}`
        // per `.github/workflows/mass-revive-strategy.yml` line 13.
        assert_eq!(
            strat.canon_name(),
            "IGLA-STRATEGY-gf64-h512-LR0.0001-rng1597-adamw"
        );
    }

    #[test]
    fn resolve_service_id_prefers_override() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev_override = env::var("SCARAB_SERVICE_ID").ok();
        let prev_railway = env::var("RAILWAY_SERVICE_ID").ok();
        env::set_var("SCARAB_SERVICE_ID", "override-id");
        env::set_var("RAILWAY_SERVICE_ID", "railway-id");
        assert_eq!(resolve_service_id().unwrap(), "override-id");
        env::remove_var("SCARAB_SERVICE_ID");
        assert_eq!(resolve_service_id().unwrap(), "railway-id");
        env::remove_var("RAILWAY_SERVICE_ID");
        if let Some(v) = prev_override {
            env::set_var("SCARAB_SERVICE_ID", v);
        }
        if let Some(v) = prev_railway {
            env::set_var("RAILWAY_SERVICE_ID", v);
        }
    }

    #[test]
    fn resolve_service_id_errors_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev_override = env::var("SCARAB_SERVICE_ID").ok();
        let prev_railway = env::var("RAILWAY_SERVICE_ID").ok();
        env::remove_var("SCARAB_SERVICE_ID");
        env::remove_var("RAILWAY_SERVICE_ID");
        assert!(resolve_service_id().is_err());
        if let Some(v) = prev_override {
            env::set_var("SCARAB_SERVICE_ID", v);
        }
        if let Some(v) = prev_railway {
            env::set_var("RAILWAY_SERVICE_ID", v);
        }
    }

    #[test]
    fn resolve_db_url_errors_when_all_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev: Vec<(&str, Option<String>)> =
            ["DATABASE_URL", "RAILWAY_POSTGRES_URL", "NEON_DATABASE_URL"]
                .iter()
                .map(|k| (*k, env::var(k).ok()))
                .collect();
        for k in ["DATABASE_URL", "RAILWAY_POSTGRES_URL", "NEON_DATABASE_URL"] {
            env::remove_var(k);
        }
        assert!(resolve_db_url().is_err());
        for (k, v) in prev {
            if let Some(v) = v {
                env::set_var(k, v);
            }
        }
    }

    #[test]
    fn parse_trainer_line_step_and_bpb() {
        let line = "step=42 val_bpb=2.5587 other=ignored";
        assert_eq!(parse_trainer_line(line), Some((42, 2.5587)));
    }

    #[test]
    fn parse_trainer_line_rejects_garbage() {
        assert_eq!(parse_trainer_line("hello world"), None);
        assert_eq!(parse_trainer_line("step=foo val_bpb=2.0"), None);
    }

    #[test]
    fn no_railway_control_plane_imports() {
        // L-SS7 / ADR-0042 invariant: the scarab runtime must NEVER
        // import or reference Railway GraphQL control-plane symbols.
        // We assert at compile time by checking that the strings do not
        // appear in this binary's own source text — a poor-man's lint
        // sufficient as a regression guard. The CI grep over
        // `.github/workflows/` is the comprehensive check; this is the
        // unit-test arm.
        let src = include_str!("scarab.rs");
        for needle in [
            "variableUpsert",
            "serviceInstanceDeployV2",
            "serviceInstanceRedeploy",
            "serviceInstanceUpdate",
            "serviceDelete",
            "RAILWAY_API_TOKEN",
            "graphql",
            "GraphQL",
        ] {
            // Allow the docstring banner to mention the names by
            // checking only for code-shaped occurrences (followed by `(`
            // or `=`). The docstring banner is in a block comment so it
            // never satisfies the call-shape predicate.
            let bad = src
                .split_whitespace()
                .any(|tok| tok.starts_with(needle) && (tok.contains('(') || tok.contains('=')));
            assert!(
                !bad,
                "scarab.rs must not invoke Railway control-plane symbol {needle:?}"
            );
        }
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

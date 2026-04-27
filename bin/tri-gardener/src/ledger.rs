//! Issue #54: tokio_postgres writer for `gardener_runs`.
//!
//! Replaces the PR-1 `println!(serde_json)` sink with a real ledger
//! write. Lookup `LedgerSink` for the production trait; tests use
//! `MockLedgerSink` so unit tests do not require a live Postgres.

use anyhow::{anyhow, Context as _, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use tokio_postgres::NoTls;

use crate::neon::projection;
use crate::state::Decision;

/// Race start anchor (T+0). Mirrors `state::RungWindow::from_now`.
const RACE_START: &str = "2026-04-27T18:00:00Z";

/// Architectural BPB floor for the trainer as currently shipped.
///
/// Champion record: `2.1919` (h=828, 2L hybrid attn, ReLU², 81K, σ²=0.0006).
/// Cross-validated against the CPU N-gram floor (~2.54) reported in
/// [trios#237](https://github.com/gHashTag/trios/issues/237) and the live
/// GPU champion tracked in [trios#143](https://github.com/gHashTag/trios/issues/143).
///
/// **Anti-cull guard:** the gardener MUST NOT issue `Decision::CullSeed`
/// for a seed whose BPB is above this floor unless plateau is
/// independently confirmed (≥5 ticks in a 0.005 band AND step ≥ 50_000).
/// Without that guard, a healthy seed sitting at the architectural floor
/// would be culled merely for not crossing the 1.85 Gate-2 target —
/// which is impossible without ALPHA's L1 / L2 / h=1024 patches landing
/// first. Gardener decision policy must read this constant rather than
/// hardcoding `2.19` at the call site.
///
/// Refs:
/// - <https://github.com/gHashTag/trios/issues/237> (CPU N-gram floor)
/// - <https://github.com/gHashTag/trios/issues/143> (GPU champion)
pub const ARCHITECTURAL_FLOOR_BPB: f64 = 2.19;

/// Single ledger row about to be written.
#[derive(Debug, Clone)]
pub struct LedgerRow {
    pub tick_t_minus: String,
    pub action: &'static str,
    pub lane: Option<String>,
    pub seed: Option<u32>,
    pub before_bpb: Option<f64>,
    pub after_bpb: Option<f64>,
    pub decision_json: serde_json::Value,
    pub outcome: Outcome,
}

/// Result of applying a single decision in Live mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Mutation skipped (review/dry-run/disabled).
    Skipped { reason: String },
    /// Mutation succeeded.
    Applied,
    /// Mutation failed with a transparent error message (R5).
    Failed { error: String },
}

impl Outcome {
    fn as_str(&self) -> &'static str {
        match self {
            Outcome::Skipped { .. } => "skipped",
            Outcome::Applied => "applied",
            Outcome::Failed { .. } => "failed",
        }
    }
}

#[async_trait]
pub trait LedgerSink: Send + Sync {
    async fn write_tick(&self, rows: &[LedgerRow]) -> Result<()>;
}

/// Production sink backed by `tokio_postgres`.
pub struct PgLedger {
    client: tokio_postgres::Client,
}

impl PgLedger {
    /// Connect using `NEON_DATABASE_URL`. Three retries with exponential
    /// backoff (250ms, 500ms, 1000ms) before giving up.
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("NEON_DATABASE_URL")
            .context("NEON_DATABASE_URL not set")?;
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..3 {
            match tokio_postgres::connect(&url, NoTls).await {
                Ok((client, connection)) => {
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            tracing::error!(?e, "neon connection task ended");
                        }
                    });
                    return Ok(Self { client });
                }
                Err(e) => {
                    let backoff_ms = 250u64 << attempt;
                    tracing::warn!(?e, attempt, backoff_ms, "neon connect failed, retrying");
                    last_err = Some(e.into());
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("neon connect: exhausted retries")))
    }
}

#[async_trait]
impl LedgerSink for PgLedger {
    async fn write_tick(&self, rows: &[LedgerRow]) -> Result<()> {
        for row in rows {
            self.client
                .execute(
                    "INSERT INTO gardener_runs \
                     (tick_t_minus, action, lane, seed, before_bpb, after_bpb, decision) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &row.tick_t_minus,
                        &row.action,
                        &row.lane,
                        &row.seed.map(|s| s as i32),
                        &row.before_bpb,
                        &row.after_bpb,
                        &row.decision_json,
                    ],
                )
                .await
                .with_context(|| format!("insert gardener_runs row action={}", row.action))?;
        }
        Ok(())
    }
}

/// In-memory sink for tests.
#[derive(Debug, Clone, Default)]
pub struct MockLedger {
    pub rows: Arc<Mutex<Vec<LedgerRow>>>,
}

#[async_trait]
impl LedgerSink for MockLedger {
    async fn write_tick(&self, rows: &[LedgerRow]) -> Result<()> {
        self.rows
            .lock()
            .map_err(|e| anyhow!("mock ledger lock poisoned: {e}"))?
            .extend_from_slice(rows);
        Ok(())
    }
}

/// Compute `T-X` style label from `now` against the race start.
pub fn tick_t_minus(now: DateTime<Utc>) -> String {
    let race_start: DateTime<Utc> = RACE_START.parse().expect("race_start parses");
    let dur = now.signed_duration_since(race_start);
    let h = dur.num_hours();
    if h < 0 {
        format!("T{:+}h", h)
    } else {
        format!("T+{}h", h)
    }
}

/// Build a `LedgerRow` from a Decision + Outcome pair.
pub fn build_row(now: DateTime<Utc>, d: &Decision, outcome: Outcome) -> LedgerRow {
    let (action, lane, seed) = projection(d);
    let mut decision_json = serde_json::to_value(d).unwrap_or(serde_json::json!({}));
    if let serde_json::Value::Object(ref mut map) = decision_json {
        map.insert(
            "outcome".to_string(),
            serde_json::Value::String(outcome.as_str().to_string()),
        );
        if let Outcome::Failed { error } = &outcome {
            map.insert(
                "error".to_string(),
                serde_json::Value::String(error.clone()),
            );
        }
    }
    LedgerRow {
        tick_t_minus: tick_t_minus(now),
        action,
        lane,
        seed,
        before_bpb: None,
        after_bpb: None,
        decision_json,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BpbStr;
    use chrono::TimeZone;

    #[test]
    fn tick_t_minus_format_after_race_start() {
        let now = Utc.with_ymd_and_hms(2026, 4, 28, 6, 0, 0).unwrap();
        assert_eq!(tick_t_minus(now), "T+12h");
    }

    #[test]
    fn tick_t_minus_format_before_race_start() {
        let now = Utc.with_ymd_and_hms(2026, 4, 27, 17, 0, 0).unwrap();
        assert_eq!(tick_t_minus(now), "T-1h");
    }

    #[test]
    fn build_row_carries_lane_and_seed_from_cull() {
        let now = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let d = Decision::CullSeed {
            lane: "L1".into(),
            seed: 210,
            bpb: BpbStr::new(2.45),
            threshold: BpbStr::new(2.30),
        };
        let row = build_row(now, &d, Outcome::Applied);
        assert_eq!(row.action, "cull");
        assert_eq!(row.lane.as_deref(), Some("L1"));
        assert_eq!(row.seed, Some(210));
        assert_eq!(row.outcome, Outcome::Applied);
        assert_eq!(
            row.decision_json["outcome"].as_str(),
            Some("applied")
        );
    }

    #[test]
    fn build_row_records_failure_error() {
        let now = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let d = Decision::CullSeed {
            lane: "L1".into(),
            seed: 211,
            bpb: BpbStr::new(2.45),
            threshold: BpbStr::new(2.30),
        };
        let row = build_row(
            now,
            &d,
            Outcome::Failed {
                error: "graphql: rate-limited".into(),
            },
        );
        assert_eq!(row.outcome.as_str(), "failed");
        assert_eq!(
            row.decision_json["error"].as_str(),
            Some("graphql: rate-limited")
        );
    }

    #[test]
    fn architectural_floor_bpb_is_2_19() {
        // Tripwire: if a future patch tries to lower this without an
        // ALPHA architecture change, this test forces the conversation.
        assert_eq!(ARCHITECTURAL_FLOOR_BPB, 2.19_f64);
    }

    #[test]
    fn architectural_floor_below_gate2_target() {
        // Sanity: floor must sit *above* the Gate-2 target. Otherwise
        // "do not cull above the floor" would be a no-op below 1.85.
        const GATE2_TARGET: f64 = 1.85;
        assert!(ARCHITECTURAL_FLOOR_BPB > GATE2_TARGET);
    }

    #[tokio::test]
    async fn mock_ledger_collects_rows() {
        let mock = MockLedger::default();
        let now = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let d = Decision::Noop {
            reason: "warmup".into(),
        };
        let row = build_row(now, &d, Outcome::Skipped { reason: "review".into() });
        mock.write_tick(&[row.clone()]).await.unwrap();
        let rows = mock.rows.lock().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "noop");
    }
}

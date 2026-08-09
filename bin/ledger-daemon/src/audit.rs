//! R7 triplet audit helper — every ledger-daemon mutation emits one row to
//! `gardener_runs`:
//!
//!   action       TEXT           e.g. 'resurrect', 'scale_up', 'leak_flag', 'zap'
//!   lane         TEXT           service name or canon prefix
//!   seed         INT (nullable)
//!   before_bpb   REAL (nullable) for leak_flag only
//!   after_bpb    REAL (nullable)
//!   decision     JSONB          {"reason":..., "target":..., "dry_run":...}
//!
//! All timestamps are server-side `now()`.

use anyhow::{Context, Result};
use serde_json::Value as Json;

pub async fn emit(
    client: &tokio_postgres::Client,
    action: &str,
    lane: &str,
    seed: Option<i32>,
    before_bpb: Option<f64>,
    after_bpb: Option<f64>,
    decision: Json,
) -> Result<()> {
    client
        .execute(
            "INSERT INTO gardener_runs (ts, action, lane, seed, before_bpb, after_bpb, decision) \
             VALUES (now(), $1, $2, $3, $4, $5, $6::jsonb)",
            &[
                &action,
                &lane,
                &seed,
                &before_bpb,
                &after_bpb,
                &decision.to_string(),
            ],
        )
        .await
        .with_context(|| format!("gardener_runs emit (action={action} lane={lane})"))?;
    tracing::info!(action, lane, ?seed, ?after_bpb, %decision, "🪲 audit");
    Ok(())
}

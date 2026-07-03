//! Four jobs of the Scarabaeus watchdog.
//!
//! PR-1 (this file) is the SCAFFOLD. Each function performs the SELECT / COUNT
//! query and emits an audit row with `decision.dry_run=true`. The actual
//! mutations (Railway redeploy, requeue UPDATE) are landed in PR-2..PR-5.

use anyhow::Result;
use serde_json::json;

use crate::audit;
use crate::config::Config;

/// Job 1 — dead-worker resurrector.
///
/// SQL:
/// ```sql
/// SELECT id, railway_acc, railway_svc_name, EXTRACT(EPOCH FROM now() - last_heartbeat)::int AS age_s
/// FROM workers
/// WHERE last_heartbeat < now() - ($1 || ' seconds')::interval
/// ```
///
/// Mutation (PR-2): Railway GraphQL `serviceInstanceRedeploy` with scope=account.
pub async fn resurrect_dead_workers(client: &tokio_postgres::Client, cfg: &Config) -> Result<()> {
    let rows = client
        .query(
            "SELECT id::text, railway_acc, railway_svc_name, \
                    EXTRACT(EPOCH FROM now() - last_heartbeat)::int8 AS age_s \
             FROM workers \
             WHERE last_heartbeat < now() - ($1::text || ' seconds')::interval \
             ORDER BY last_heartbeat ASC \
             LIMIT 100",
            &[&cfg.worker_dead_after_secs.to_string()],
        )
        .await?;

    for row in &rows {
        let id: String = row.get(0);
        let acc: Option<String> = row.try_get(1).ok();
        let svc: Option<String> = row.try_get(2).ok();
        let age_s: i64 = row.get(3);

        audit::emit(
            client,
            "resurrect_candidate",
            svc.as_deref().unwrap_or("unknown"),
            None,
            None,
            None,
            json!({
                "worker_id": id,
                "account": acc,
                "heartbeat_age_s": age_s,
                "threshold_s": cfg.worker_dead_after_secs,
                "dry_run": true,
                "todo_pr_2": "Railway serviceInstanceRedeploy",
            }),
        )
        .await?;
    }

    if !rows.is_empty() {
        tracing::warn!(count = rows.len(), "dead workers detected (dry-run)");
    }
    Ok(())
}

/// Job 2 — autoscaler.
///
/// SQL:
/// ```sql
/// SELECT count(*) FROM experiment_queue
/// WHERE status='pending' AND scheduled_at <= now()
/// ```
///
/// Decision (PR-3): `desired = clamp(queue_depth / 10, MIN_REPLICAS, MAX_REPLICAS)`
/// and patch Railway service replicas accordingly.
pub async fn autoscale(client: &tokio_postgres::Client, cfg: &Config) -> Result<()> {
    let row = client
        .query_one(
            "SELECT count(*) FROM experiment_queue \
             WHERE status='pending' AND scheduled_at <= now()",
            &[],
        )
        .await?;
    let depth: i64 = row.get(0);
    let desired = (depth / 10)
        .max(cfg.min_replicas as i64)
        .min(cfg.max_replicas as i64) as u32;

    audit::emit(
        client,
        "autoscale_compute",
        "seed-agent-pool",
        None,
        None,
        None,
        json!({
            "queue_depth": depth,
            "desired_replicas": desired,
            "min": cfg.min_replicas,
            "max": cfg.max_replicas,
            "dry_run": true,
            "todo_pr_3": "Railway service replicas patch",
        }),
    )
    .await?;
    Ok(())
}

/// Job 3 — leak gate.
///
/// SQL:
/// ```sql
/// UPDATE experiment_queue SET last_error = 'SCARABAEUS-LEAK-CANDIDATE: bpb<threshold'
/// WHERE status='done' AND final_bpb < $1 AND (last_error IS NULL OR last_error NOT LIKE 'SCARABAEUS-%')
/// RETURNING id, canon_name, final_bpb
/// ```
pub async fn leak_gate(client: &tokio_postgres::Client, cfg: &Config) -> Result<()> {
    let rows = client
        .query(
            "UPDATE experiment_queue \
             SET last_error = 'SCARABAEUS-LEAK-CANDIDATE: bpb<' || $1 \
             WHERE status='done' \
               AND final_bpb IS NOT NULL \
               AND final_bpb < $1 \
               AND (last_error IS NULL OR last_error NOT LIKE 'SCARABAEUS-%') \
             RETURNING id, canon_name, final_bpb",
            &[&cfg.leak_bpb_threshold],
        )
        .await?;

    for row in &rows {
        let id: i64 = row.get(0);
        let canon: String = row.get(1);
        let bpb: f64 = row.get(2);
        audit::emit(
            client,
            "leak_flag",
            &canon,
            None,
            Some(bpb),
            Some(bpb),
            json!({
                "queue_id": id,
                "threshold": cfg.leak_bpb_threshold,
                "reason": "bpb below subfloor; requires heldout eval (Khepri-4)",
            }),
        )
        .await?;
    }

    if !rows.is_empty() {
        tracing::warn!(count = rows.len(), "leak candidates flagged");
    }
    Ok(())
}

/// Job 4 — stuck-running zapper.
///
/// Requeues rows that have been `status='running'` longer than
/// `stuck_running_after_hours` via Khepri-2 semantics (retry with backoff).
pub async fn zap_stuck_running(client: &tokio_postgres::Client, cfg: &Config) -> Result<()> {
    let rows = client
        .query(
            "UPDATE experiment_queue \
             SET status = CASE WHEN attempts + 1 >= max_attempts THEN 'dead' ELSE 'pending' END, \
                 attempts = attempts + 1, \
                 last_error = 'SCARABAEUS-ZAP-STUCK: running>' || $1 || 'h', \
                 scheduled_at = now() + (power(2, attempts) * interval '1 minute'), \
                 worker_id = NULL, \
                 claimed_at = NULL, \
                 started_at = NULL, \
                 finished_at = CASE WHEN attempts + 1 >= max_attempts THEN now() ELSE NULL END \
             WHERE status='running' \
               AND started_at < now() - ($1::text || ' hours')::interval \
             RETURNING id, canon_name, attempts",
            &[&cfg.stuck_running_after_hours.to_string()],
        )
        .await?;

    for row in &rows {
        let id: i64 = row.get(0);
        let canon: String = row.get(1);
        let attempts: i32 = row.get(2);
        audit::emit(
            client,
            "zap_stuck",
            &canon,
            None,
            None,
            None,
            json!({
                "queue_id": id,
                "attempts_after": attempts,
                "threshold_hours": cfg.stuck_running_after_hours,
            }),
        )
        .await?;
    }

    if !rows.is_empty() {
        tracing::warn!(count = rows.len(), "stuck running jobs zapped");
    }
    Ok(())
}

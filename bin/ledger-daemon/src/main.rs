//! ledger-daemon — Scarabaeus Engine watchdog (Khepri-3).
//!
//! One tick every `LEDGER_TICK_SECS` (default 30 s). Four jobs per tick:
//!
//!   1. Dead-worker resurrector: `workers.last_heartbeat` older than
//!      `WORKER_DEAD_AFTER_SECS` → Railway redeploy.
//!   2. Autoscaler: `queue_depth = COUNT(experiment_queue WHERE status='pending'
//!      AND scheduled_at <= now())` → target N replicas.
//!   3. Leak gate: `experiment_queue` rows landing `done` with `bpb < 0.1`
//!      get `last_error='SCARABAEUS-LEAK-CANDIDATE'`.
//!   4. Stuck-job zapper: rows `status='running' AND started_at <
//!      now() - interval '1 hour'` are requeued via Khepri-2 semantics.
//!
//! All mutations emit one audit row into `gardener_runs` with the R7 triplet.
//!
//! **STATUS: PR-1 scaffold**. Each job is a TODO stub with the exact SQL it
//! will run — wires up postgres client, tick loop, graceful shutdown, and
//! audit emit helper. The four jobs are implemented in PR-2..PR-5, one per
//! sub-issue of [trios-railway#101](https://github.com/gHashTag/trios-railway/issues/101).
//!
//! Anchor: `phi^2 + phi^-2 = 3`.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::signal;
use tokio::time::{interval, MissedTickBehavior};
use tokio_postgres::NoTls;

mod audit;
mod config;
mod jobs;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env().context("load config")?;
    tracing::info!(?cfg, "🪲 ledger-daemon starting (Khepri-3 watchdog)");

    // Connect to Neon. Session-mode pooler recommended for long-lived
    // connections; transaction-mode will work but we re-connect on error.
    let (client, connection) = tokio_postgres::connect(&cfg.neon_dsn, NoTls)
        .await
        .context("neon connect")?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!(?e, "neon connection task died");
        }
    });
    tracing::info!("neon connection OK");

    let mut tick = interval(Duration::from_secs(cfg.tick_secs));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if let Err(e) = tick_once(&client, &cfg).await {
                    tracing::warn!(?e, "tick failed (non-fatal, will retry next tick)");
                }
            }
            _ = signal::ctrl_c() => {
                tracing::info!("shutdown signal received; exiting cleanly");
                break;
            }
        }
    }

    Ok(())
}

async fn tick_once(client: &tokio_postgres::Client, cfg: &Config) -> Result<()> {
    // Each job returns a Result<()>; we collect errors but never bail the
    // whole tick — one failing job must not starve the other three.
    let mut errors: Vec<String> = Vec::new();

    if let Err(e) = jobs::resurrect_dead_workers(client, cfg).await {
        errors.push(format!("resurrect: {e:#}"));
    }
    if let Err(e) = jobs::autoscale(client, cfg).await {
        errors.push(format!("autoscale: {e:#}"));
    }
    if let Err(e) = jobs::leak_gate(client, cfg).await {
        errors.push(format!("leak_gate: {e:#}"));
    }
    if let Err(e) = jobs::zap_stuck_running(client, cfg).await {
        errors.push(format!("zap_stuck: {e:#}"));
    }

    if errors.is_empty() {
        tracing::info!("tick ok");
    } else {
        tracing::warn!(?errors, "tick completed with job errors");
    }
    Ok(())
}

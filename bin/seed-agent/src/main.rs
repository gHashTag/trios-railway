//! `seed-agent` — pull-based trainer worker for ADR-0081 unified
//! experiment loop ([trios-railway#81](https://github.com/gHashTag/trios-railway/issues/81)).
//!
//! Architecture (one-line):
//!
//! ```text
//! Neon experiment_queue → seed-agent claim → trainer run → bpb_samples → early-stop @ 1000 → loop
//! ```
//!
//! Standing rules:
//!   R1 — Rust-only.
//!   R5 — Honest exit codes; every Neon error is logged and surfaces.
//!   R7 — Audit triplet emitted via experience log on every claim.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`.

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::time::sleep;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod claim;
mod early_stop;
mod telemetry;
mod trainer;
mod worker;

#[derive(Debug, Parser)]
#[command(name = "seed-agent", version, about = "ADR-0081 pull-based trainer worker")]
struct Cli {
    /// Railway account this worker runs in (`acc0..acc3`).
    #[arg(long, env = "RAILWAY_ACC")]
    railway_acc: String,

    /// Railway service ID hosting this worker (recorded in `workers`).
    #[arg(long, env = "RAILWAY_SERVICE_ID")]
    railway_svc_id: String,

    /// Railway service name (human-readable label).
    #[arg(long, env = "RAILWAY_SERVICE_NAME")]
    railway_svc_name: String,

    /// Idle sleep between empty-queue polls.
    #[arg(long, default_value_t = 30)]
    poll_idle_secs: u64,

    /// Step at which the early-stop decision is made.
    #[arg(long, default_value_t = 1000)]
    early_stop_step: u32,

    /// Early-stop BPB ceiling. Exceeding this at `early_stop_step` →
    /// abandon experiment, mark `pruned`, pull next.
    #[arg(long, default_value_t = 2.60)]
    early_stop_bpb_ceiling: f64,

    /// `NEON_DATABASE_URL`. Required.
    #[arg(long, env = "NEON_DATABASE_URL")]
    neon_url: String,

    /// Trainer kind. `mock` runs the deterministic in-process simulator
    /// (used in CI / local smoke). `external` shells out to the IGLA
    /// trainer (gated behind a future feature flag).
    #[arg(long, default_value = "mock")]
    trainer_kind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!(
        "[seed-agent] boot pid={} acc={} svc={}",
        std::process::id(),
        std::env::var("RAILWAY_ACC").unwrap_or_else(|_| "?".to_string()),
        std::env::var("RAILWAY_SERVICE_ID").unwrap_or_else(|_| "?".to_string()),
    );

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stdout)
        .compact()
        .init();

    let cli = Cli::parse();

    let cfg = worker::WorkerConfig {
        worker_id: Uuid::new_v4(),
        railway_acc: cli.railway_acc.clone(),
        railway_svc_id: cli.railway_svc_id.clone(),
        railway_svc_name: cli.railway_svc_name.clone(),
        poll_idle: Duration::from_secs(cli.poll_idle_secs),
        early_stop_step: cli.early_stop_step,
        early_stop_bpb_ceiling: cli.early_stop_bpb_ceiling,
        trainer_kind: cli.trainer_kind.clone(),
    };

    tracing::info!(
        worker_id = %cfg.worker_id,
        acc = %cfg.railway_acc,
        svc = %cfg.railway_svc_id,
        "seed-agent starting pull loop"
    );

    let (client, conn) = tokio_postgres::connect(&cli.neon_url, tokio_postgres::NoTls)
        .await
        .with_context(|| "connect to NEON_DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::error!(?e, "neon connection lost");
        }
    });

    worker::register_worker(&client, &cfg)
        .await
        .with_context(|| "register worker in `workers`")?;

    // Graceful shutdown on SIGTERM / SIGINT (Railway sends SIGTERM on
    // redeploy; honor it so the claimed row is released cleanly).
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("shutdown signal — releasing claimed experiment if any");
                worker::release_on_shutdown(&client, &cfg).await.ok();
                break;
            }
            r = worker::run_one_iteration(&client, &cfg) => {
                match r {
                    Ok(worker::IterOutcome::Trained(canon)) => {
                        tracing::info!(canon = %canon, "experiment finished — pulling next");
                    }
                    Ok(worker::IterOutcome::Pruned(canon, reason)) => {
                        tracing::info!(canon = %canon, %reason, "early-stop pruned — pulling next");
                    }
                    Ok(worker::IterOutcome::Idle) => {
                        sleep(cfg.poll_idle).await;
                    }
                    Err(e) => {
                        tracing::error!(?e, "iteration failed; backing off 30s and retrying");
                        sleep(Duration::from_secs(30)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

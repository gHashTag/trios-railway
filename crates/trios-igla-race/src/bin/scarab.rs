//! Stateless fungible worker — entry point for Railway scarab pool
//! All logic lives in trios_igla_race::* modules

use anyhow::{Result, Context};
use clap::Parser;
use tracing::{info, warn, error};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

/// Scarab Worker CLI
#[derive(Parser, Debug)]
#[command(
    name = "scarab",
    version = "0.1.0",
    about = "Scarab Worker — SQL-driven experiment runner for IGLA RACE"
)]
struct Cli {
    /// Neon database URL
    #[arg(long, env = "NEON_DATABASE_URL")]
    neon_url: String,

    /// Railway account this worker belongs to (acc0..acc5)
    #[arg(long, env = "SCARAB_ACCOUNT", default_value = "acc0")]
    account: String,
}

/// Path to trainer binary
const TRAINER_BIN: &str = "/usr/local/bin/trios-train";

/// Training step output from trainer (JSONL)
#[derive(Debug, Clone)]
struct TrainerStep {
    step: u32,
    loss: f64,
    bpb: Option<f64>,
}

impl TrainerStep {
    fn parse(line: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        Some(TrainerStep {
            step: v.get("step")?.as_u64()? as u32,
            loss: v.get("loss")?.as_f64()?,
            bpb: v.get("bpb").and_then(|x| x.as_f64()),
        })
    }
}

/// Run trainer and collect results
async fn run_trainer(seed: i32, steps: usize, lr: f64, hidden: usize) -> Result<(f64, u32)> {
    tracing::info!(
        seed = seed,
        steps = steps,
        lr = lr,
        hidden = hidden,
        "Starting trainer"
    );

    let mut cmd = Command::new(TRAINER_BIN);
    cmd.arg("--seed")
        .arg(seed.to_string())
        .arg("--steps")
        .arg(steps.to_string())
        .arg("--lr")
        .arg(lr.to_string())
        .arg("--hidden")
        .arg(hidden.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().context("failed to spawn trainer")?;

    let stdout = child.stdout.take().context("no stdout")?;
    let stderr = child.stderr.take().context("no stderr")?;

    // Parse stdout line by line
    let mut reader = BufReader::new(stdout);
    let mut last_step: Option<TrainerStep> = None;
    let mut steps_seen = 0u32;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Some(step) = TrainerStep::parse(trimmed) {
                steps_seen += 1;
                last_step = Some(step);
            }
        }
        line.clear();
    }

    let exit_status = child.wait().await?;

    if !exit_status.success() {
        let mut stderr_reader = BufReader::new(stderr);
        let mut stderr_buf = String::new();
        while stderr_reader.read_line(&mut stderr_buf).await? > 0 {}
        anyhow::bail!(
            "trainer exited with status {}: {}",
            exit_status,
            stderr_buf
        );
    }

    let last_bpb = last_step.as_ref()
        .and_then(|s| s.bpb)
        .context("no BPB in trainer output")?;

    let final_step = last_step.as_ref()
        .map(|s| s.step)
        .context("no steps in trainer output")?;

    tracing::info!(
        final_bpb = last_bpb,
        final_step,
        steps_seen,
        "Training completed"
    );

    Ok((last_bpb, final_step))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "scarab=info,tokio_postgres=warn".to_string()),
        )
        .init();

    let cli = Cli::parse();
    let worker_id = Uuid::new_v4();

    info!("🪲 Scarab Worker starting");
    info!("  Account: {}", cli.account);
    info!("  Worker ID: {}", worker_id);

    use trios_igla_race::pull_queue;

    // Connect to database
    let db = pull_queue::PullQueueDb::connect(&cli.neon_url).await?;

    // Register this worker
    db.register_worker(&worker_id, &cli.account, "scarab-pool").await?;

    // Spawn heartbeat task
    let heartbeat_db = db.clone_handle();
    let heartbeat_worker_id = worker_id;
    let _heartbeat_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            if let Err(e) = heartbeat_db.update_heartbeat(&heartbeat_worker_id, None).await {
                warn!("heartbeat failed for {}: {e}", heartbeat_worker_id);
            }
        }
    });

    // Main scarab loop — claim → train → done
    loop {
        // Step 1: claim
        let maybe_exp = match db.pull_experiment(&worker_id).await {
            Ok(exp) => exp,
            Err(e) => {
                error!(error = %e, "Claim failed, retrying in 10s");
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }
        };

        match maybe_exp {
            Some(exp) => {
                info!("🎯 Claimed: {} (id={})", exp.canon_name, exp.id);

                // Parse config from JSON
                let config = pull_queue::ExperimentConfig::from_value(&exp.config_json)?;

                // Step 2: mark as running
                if let Err(e) = db.mark_running(exp.id).await {
                    error!(error = %e, "Failed to mark running");
                    continue;
                }

                // Step 3: run trainer
                let result = run_trainer(exp.seed, config.steps, config.lr, config.hidden).await;

                match result {
                    Ok((final_bpb, final_step)) => {
                        // Check BPB threshold (5% of steps as heuristic)
                        let kill_threshold = config.steps as f64 * 0.05;
                        if final_bpb > kill_threshold {
                            warn!(
                                exp_id = exp.id,
                                bpb = final_bpb,
                                threshold = kill_threshold,
                                "BPB exceeded threshold, marking failed"
                            );
                            db.mark_abandoned(exp.id, &format!("BPB {} > {}", final_bpb, kill_threshold)).await?;
                        } else {
                            info!(
                                exp_id = exp.id,
                                bpb = final_bpb,
                                step = final_step,
                                "Marking done"
                            );
                            db.mark_done(exp.id, final_bpb as f32, final_step as i32).await?;
                        }
                    }
                    Err(e) => {
                        error!(exp_id = exp.id, error = %e, "Training failed");
                        db.mark_abandoned(exp.id, &e.to_string()).await?;
                    }
                }
            }
            None => {
                tracing::debug!("No pending work, sleeping...");
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        }
    }
}

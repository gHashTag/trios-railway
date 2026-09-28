//! Database-driven scarab binary.
//!
//! Environment:
//!   DATABASE_URL         - Required: database connection string
//!   SCARAB_SERVICE_ID   - Optional: unique scarab identifier (falls back to RAILWAY_SERVICE_NAME)
//!   SCARAB_POLL_INTERVAL - Optional: seconds between polls (default: 10)

use anyhow::Context;
use clap::Parser;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

use trios_igla_race::scarab_polling::{ScarabStrategy, TrainingProcess};

#[derive(Parser, Debug)]
struct Cli {
    /// Database connection string
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Unique service identifier for this scarab (defaults to RAILWAY_SERVICE_NAME)
    #[arg(long, env = "SCARAB_SERVICE_ID")]
    service_id: Option<String>,

    /// Poll interval in seconds
    #[arg(long, env = "SCARAB_POLL_INTERVAL", default_value = "10")]
    poll_interval: u64,
}

const STRATEGY_SQL: &str = r#"
    SELECT service_id, optimizer, hidden, lr, seed, steps, ctx,
           format, status, strategy_hash, canon_name
    FROM public.scarab_strategy
    WHERE service_id = $1
"#;

const HEARTBEAT_SQL: &str = r#"
    INSERT INTO public.scarab_heartbeat
        (service_id, is_training, current_hash, last_heartbeat, trainer_pid, run_started_at)
    VALUES ($1, $2, $3, NOW(), $4, $5)
    ON CONFLICT (service_id) DO UPDATE SET
        is_training = EXCLUDED.is_training,
        current_hash = EXCLUDED.current_hash,
        last_heartbeat = NOW(),
        trainer_pid = EXCLUDED.trainer_pid,
        run_started_at = EXCLUDED.run_started_at
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "scarab_strategy=info,tokio_postgres=warn".to_string()),
        )
        .init();

    let cli = Cli::parse();

    // Get service_id from SCARAB_SERVICE_ID or RAILWAY_SERVICE_NAME
    let service_id = cli.service_id
        .or_else(|| std::env::var("RAILWAY_SERVICE_NAME").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .ok_or_else(|| anyhow::anyhow!(
            "Service ID required: set SCARAB_SERVICE_ID, RAILWAY_SERVICE_NAME, or HOSTNAME"
        ))?;

    info!("🪲 Database-driven Scarab starting");
    info!("  Service ID: {}", service_id);
    info!("  Poll interval: {}s", cli.poll_interval);

    // Connect to database
    let mut builder = SslConnector::builder(SslMethod::tls())?;
    builder.set_verify(SslVerifyMode::NONE);
    let connector = MakeTlsConnector::new(builder.build());
    let (client, conn) = tokio_postgres::connect(&cli.database_url, connector)
        .await
        .context("Failed to connect to database")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            error!("Connection error: {e}");
        }
    });

    // Verify strategy exists for this service_id
    let strategy_row = client.query_opt(STRATEGY_SQL, &[&service_id]).await?;
    if strategy_row.is_none() {
        anyhow::bail!(
            "No strategy found for service_id='{}'. \
             Create a row in public.scarab_strategy first.",
            service_id
        );
    }

    let mut training: Option<TrainingProcess> = None;
    let mut last_hash: Option<String> = None;
    let mut run_started_at: Option<chrono::DateTime<chrono::Utc>> = None;

    loop {
        // Fetch current strategy
        let row = match client.query_opt(STRATEGY_SQL, &[&service_id]).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                warn!("Strategy deleted for service_id='{}'", service_id);
                if let Some(mut t) = training.take() {
                    let _ = t.stop().await;
                }
                sleep(Duration::from_secs(cli.poll_interval)).await;
                continue;
            }
            Err(e) => {
                error!("Query failed: {e}");
                sleep(Duration::from_secs(cli.poll_interval)).await;
                continue;
            }
        };

        let strategy = ScarabStrategy {
            service_id: row.get(0),
            optimizer: row.get(1),
            hidden: row.get(2),
            lr: row.get(3),
            seed: row.get(4),
            steps: row.get(5),
            ctx: row.get(6),
            format: row.get(7),
            status: row.get(8),
            strategy_hash: row.get(9),
            canon_name: row.get(10),
        };

        let current_hash = strategy.strategy_hash.clone();
        let needs_restart = strategy.requires_restart(last_hash.as_deref());

        if needs_restart {
            info!(
                "Strategy changed: status={}, hash={}",
                strategy.status,
                if current_hash.len() > 12 {
                    &current_hash[..12]
                } else {
                    &current_hash
                }
            );

            // Stop current training if running
            if let Some(mut t) = training.take() {
                info!("Stopping previous training...");
                if let Err(e) = t.stop().await {
                    warn!("Stop error: {e}");
                }
                run_started_at = None;
            }

            // Start new training if status='active'
            if strategy.status == "active" {
                let exp_id = format!("{}-{}", service_id, chrono::Utc::now().timestamp());
                let args = strategy.train_args(&exp_id);

                match TrainingProcess::start(&exp_id, &args) {
                    Ok(t) => {
                        info!("Started new training (exp_id={})", exp_id);
                        training = Some(t);
                        run_started_at = Some(chrono::Utc::now());
                    }
                    Err(e) => {
                        error!("Failed to start training: {e}");
                    }
                }
            } else {
                info!("Status is '{}', not starting training", strategy.status);
            }

            last_hash = Some(current_hash);
        }

        // Update heartbeat
        let is_training = if let Some(t) = training.as_mut() {
            t.is_running()
        } else {
            false
        };
        let trainer_pid = training.as_ref().and_then(|_| {
            training.as_ref().unwrap().pid()
        });
        let run_start_ts: Option<chrono::DateTime<chrono::Utc>> = run_started_at;

        if let Err(e) = client.execute(
            HEARTBEAT_SQL,
            &[
                &service_id,
                &is_training,
                &last_hash,
                &trainer_pid,
                &run_start_ts,
            ],
        ).await {
            warn!("Heartbeat update failed: {e}");
        }

        sleep(Duration::from_secs(cli.poll_interval)).await;
    }
}

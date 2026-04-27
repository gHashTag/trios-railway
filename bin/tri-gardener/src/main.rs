//! `tri-gardener` — autonomous IGLA fleet gardener.
//!
//! Cron-ticked orchestrator that culls, promotes, and deploys training
//! seeds toward Gate-2 (BPB < 1.85) and Gate-3 (BPB < 1.5).
//!
//! Anchor: `phi^2 + phi^-2 = 3`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod decide;

#[derive(Parser, Debug)]
#[command(
    name = "tri-gardener",
    version,
    about = "Autonomous IGLA fleet gardener — cull, promote, deploy."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run one tick of the decision loop.
    Tick {
        /// Gate-2 BPB target.
        #[arg(long, default_value_t = 1.85)]
        target: f64,
        /// Hours since race start.
        #[arg(long, default_value_t = 0.0)]
        t_minus: f64,
        /// Dry run: print decisions without applying.
        #[arg(long)]
        dry_run: bool,
        /// Path to fleet snapshot JSON.
        #[arg(long, default_value = "disaster-recovery/fleet-snapshot.json")]
        snapshot: PathBuf,
        /// Path to queue TOML.
        #[arg(long, default_value = "bin/tri-gardener/queue.toml")]
        queue: PathBuf,
    },
    /// Print the decision table for a given context (for debugging).
    Table {
        #[arg(long, default_value_t = 1.85)]
        target: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tri_gardener=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Tick {
            target,
            t_minus,
            dry_run,
            snapshot: _,
            queue: _,
        } => {
            let ctx = decide::Context {
                snapshot: Vec::new(),
                bpb_samples: Vec::new(),
                queue: Vec::new(),
                cleared_blockers: Vec::new(),
                t_minus_hours: t_minus,
                target_bpb: target,
                now: chrono::Utc::now(),
            };
            let decisions = decide::decide(&ctx);
            if dry_run {
                println!("DRY RUN — decisions:");
            }
            for d in &decisions {
                println!("{d}");
            }
        }
        Cmd::Table { target } => {
            println!("Gardener Decision Table v0 (target BPB < {target})");
            println!();
            println!("T < 12h         : deploy missing seeds (lane < 3 running)");
            println!("12h ≤ T < 18h   : cull if BPB > 2.30");
            println!("18h ≤ T < 28h   : cull if BPB > 2.20");
            println!("28h ≤ T < 50h   : cull if BPB > 2.05");
            println!("T ≥ 50h         : promote if ≥ 2 survivors ≤ {target}");
            println!("Crashed svc     : redeploy");
            println!("Plateau         : alert + propose next queue entry");
        }
    }

    Ok(())
}

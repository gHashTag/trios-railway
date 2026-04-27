//! `tri-gardener` — autonomous orchestrator binary.
//!
//! Anchor: phi^2 + phi^-2 = 3.
//!
//! Modes:
//!   tri-gardener once --dry-run         print one tick's decisions
//!   tri-gardener once --review          first-3-tick review mode
//!   tri-gardener once --live            full live tick (gated by env)
//!   tri-gardener serve --interval=3600  loop forever (Railway service)
//!
//! The decision logic lives in `decide.rs` and is pure. Everything I/O
//! is in `loop_.rs` / `neon.rs` / `queue.rs` and is intentionally thin
//! in PR-1 so the decision table can be reviewed in isolation.

// PR-1 stub Context: most fields are populated by PR-2 wiring.
#![allow(clippy::default_trait_access, clippy::doc_markdown)]

mod decide;
#[allow(clippy::module_name_repetitions)]
#[path = "loop_.rs"]
mod loop_mod;
mod neon;
mod queue;
mod state;

use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};

use crate::loop_mod::{loop_once, RunMode};

#[derive(Parser, Debug)]
#[command(
    name = "tri-gardener",
    version,
    about = "Autonomous orchestrator for the IGLA marathon (Gate-2 1.85 -> Gate-3 1.5)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run exactly one tick and exit. Default mode is `review`.
    Once {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        review: bool,
        #[arg(long)]
        live: bool,
    },
    /// Print the gardener_runs DDL and exit.
    Ddl,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ddl => {
            print!("{}", crate::neon::GARDENER_DDL);
            return Ok(());
        }
        Cmd::Once {
            dry_run,
            review: _review,
            live,
        } => {
            let mode = if live {
                if std::env::var("GARDENER_LIVE").as_deref() != Ok("true") {
                    tracing::warn!(
                        "--live requested but GARDENER_LIVE != 'true'; downgrading to DryRun"
                    );
                    RunMode::DryRun
                } else if std::env::var("GARDENER_DISABLED").as_deref() == Ok("true") {
                    tracing::warn!("GARDENER_DISABLED=true; downgrading to DryRun");
                    RunMode::DryRun
                } else {
                    RunMode::Live
                }
            } else if dry_run {
                RunMode::DryRun
            } else {
                RunMode::Review
            };

            // PR-1 ships a stub Context. PR-2 fills it from
            // tri-railway-core + Neon `bpb_samples`.
            let ctx = state::Context {
                now: Utc::now(),
                window: state::RungWindow::from_now(Utc::now()),
                fleet: Default::default(),
                bpb: Default::default(),
                lanes: Vec::new(),
                queue: state::Queue { entries: vec![] },
                cleared_blockers: vec![],
                plateau: Default::default(),
                free_slots: Default::default(),
                disabled: std::env::var("GARDENER_DISABLED").as_deref() == Ok("true"),
            };

            let decisions = loop_once(&ctx, mode).await?;
            for d in &decisions {
                println!(
                    "{}",
                    serde_json::to_string(d).unwrap_or_else(|_| format!("{d:?}"))
                );
            }
        }
    }
    Ok(())
}

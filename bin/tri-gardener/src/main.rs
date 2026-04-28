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

mod actuate;
mod bpb_source;
mod decide;
mod leaderboard;
mod ledger;
#[allow(clippy::module_name_repetitions)]
#[path = "loop_.rs"]
mod loop_mod;
mod neon;
mod queue;
mod serve;
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
    /// Long-lived service mode. Drives one tick every --interval seconds.
    Serve {
        /// Tick interval in seconds. Range: 60..=86_400. Default 3600.
        #[arg(long, default_value_t = 3600)]
        interval: u64,
        /// Run mode: review | dry-run | live. Default review.
        #[arg(long, default_value = "review")]
        mode: String,
    },
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
        Cmd::Serve { interval, mode } => {
            let dur = crate::serve::validate_interval(interval)?;
            tracing::info!(interval_secs = interval, %mode, "tri-gardener serve starting");

            // Stop flag toggled by SIGINT/SIGTERM.
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::sync::Arc;
            let stop = Arc::new(AtomicBool::new(false));
            {
                let stop = stop.clone();
                tokio::spawn(async move {
                    if let Ok(()) = tokio::signal::ctrl_c().await {
                        tracing::warn!("ctrl_c received, requesting graceful stop");
                        stop.store(true, Ordering::SeqCst);
                    }
                });
            }

            // Per-tick body: re-runs `Once` logic in the chosen mode.
            struct OnceTick {
                mode: String,
            }
            #[async_trait::async_trait]
            impl crate::serve::TickHandler for OnceTick {
                async fn on_tick(&self, idx: u64) -> Result<()> {
                    tracing::info!(idx, mode = %self.mode, "serve tick");
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
                    let mode = match self.mode.as_str() {
                        "live" => RunMode::DryRun, // gated; real Live arm uses loop_once_live
                        "dry-run" => RunMode::DryRun,
                        _ => RunMode::Review,
                    };
                    let _ = loop_once(&ctx, mode).await?;
                    Ok(())
                }
            }
            let handler = OnceTick { mode: mode.clone() };
            let stop_check = {
                let stop = stop.clone();
                move || stop.load(Ordering::SeqCst)
            };
            let ticks = crate::serve::serve_loop(dur, &handler, stop_check).await?;
            tracing::info!(ticks, "tri-gardener serve exited cleanly");
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

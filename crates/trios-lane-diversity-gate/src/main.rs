//! # trios-lane-diversity-gate
//!
//! R7 falsifiability gate for the IGLA marathon. Closes
//! [gHashTag/trios#774](https://github.com/gHashTag/trios/issues/774).
//!
//! ## Why
//!
//! Before the gardener declares any `champion_*` row in
//! `ssot.bpb_samples`, the fleet MUST have run on:
//!
//! - **≥ 3 distinct learning rates** (LR perturbation ablation)
//! - **≥ 3 distinct seeds** (rng variance ablation)
//! - **≥ 1 baseline lane** (so LANE registry isn't a single lane)
//!
//! Otherwise the "champion" is unfalsifiable: a single seed × single LR
//! result is consistent with seed-lottery (a different rng would produce
//! a different winner).
//!
//! This gate parses the canonical name pattern
//! `IGLA-{LANE}-{format}-h{H}-LR{L}-rng{SEED}-{algo}` and counts unique
//! axes from `ssot.bpb_samples` over a configurable window (default 6h).
//!
//! ## Usage
//!
//! ```text
//! trios-lane-diversity-gate \
//!     --neon-url "$RAILWAY_POSTGRES_URL" \
//!     --window-hours 6 \
//!     --min-lr 3 --min-rng 3 --min-lane 1
//! ```
//!
//! Exit codes:
//! * **0** → all diversity thresholds met (R7 GREEN) — gardener may
//!   declare champion
//! * **1** → diversity thresholds violated (R7 RED) — gardener MUST
//!   suppress champion_* writes until fleet expands
//! * **2** → infrastructure error (DB unreachable, no rows at all) —
//!   treat as RED (fail-closed)
//!
//! ## R5 contract
//!
//! Every invocation emits a JSON line to stdout with:
//! `{verdict, uniq_lr, uniq_rng, uniq_lane, uniq_canon, rows_window,
//!   window_hours, thresholds, evidence_sql}`
//!
//! This stdout is intended to be captured by `tri-gardener` Stage 0e
//! and written to `audit_runs(probe='diversity_gate', verdict, evidence)`.
//!
//! ## Sibling skills
//!
//! - `leaderboard-snapshot` v1.0 owns the canon-name regex.
//! - `igla-honest-short-run` v1.0 is the per-trainer pre-flight gate.
//! - `tri-gardener-runbook` v2.9 must call this binary in Stage 0e.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::process::ExitCode;
use tokio_postgres::NoTls;
use tracing::{info, warn};

/// CLI options.
#[derive(Parser, Debug)]
#[command(name = "trios-lane-diversity-gate", version)]
struct Cli {
    /// Postgres URL (Railway `phd-postgres-ssot`).
    #[arg(long, env = "RAILWAY_POSTGRES_URL")]
    neon_url: String,

    /// Time window to count diversity over.
    #[arg(long, default_value_t = 6)]
    window_hours: u32,

    /// Minimum unique LR values required.
    #[arg(long, default_value_t = 3)]
    min_lr: i64,

    /// Minimum unique seed values required.
    #[arg(long, default_value_t = 3)]
    min_rng: i64,

    /// Minimum unique LANE values required.
    #[arg(long, default_value_t = 1)]
    min_lane: i64,

    /// Minimum total rows in window (below this → infra error).
    #[arg(long, default_value_t = 50)]
    min_rows: i64,
}

/// JSON shape emitted to stdout — consumed by gardener Stage 0e.
#[derive(Serialize)]
struct Verdict {
    verdict: &'static str,
    uniq_lr: i64,
    uniq_rng: i64,
    uniq_lane: i64,
    uniq_canon: i64,
    rows_window: i64,
    window_hours: u32,
    thresholds: Thresholds,
    evidence_sql: &'static str,
    anomalies: Vec<String>,
}

#[derive(Serialize)]
struct Thresholds {
    min_lr: i64,
    min_rng: i64,
    min_lane: i64,
    min_rows: i64,
}

const EVIDENCE_SQL: &str = r#"
SELECT
  count(DISTINCT canon_name) AS uniq_canon,
  count(DISTINCT split_part(split_part(canon_name,'LR',2),'-',1)) AS uniq_lr,
  count(DISTINCT split_part(split_part(canon_name,'rng',2),'-',1)) AS uniq_rng,
  count(DISTINCT split_part(canon_name,'-',2)) AS uniq_lane,
  count(*) AS rows_window
FROM ssot.bpb_samples
WHERE ts > now() - ($1 || ' hours')::interval
  AND canon_name LIKE 'IGLA-%'
"#;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "trios_lane_diversity_gate=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match run(&cli).await {
        Ok(verdict) => {
            // R5: always emit JSON line to stdout, even on GREEN
            match serde_json::to_string(&verdict) {
                Ok(j) => println!("{j}"),
                Err(e) => eprintln!("FATAL serialize: {e}"),
            }
            match verdict.verdict {
                "green" => ExitCode::from(0),
                "red" => ExitCode::from(1),
                _ => ExitCode::from(2),
            }
        }
        Err(e) => {
            warn!("infrastructure error: {e:#}");
            let v = Verdict {
                verdict: "infra_error",
                uniq_lr: -1,
                uniq_rng: -1,
                uniq_lane: -1,
                uniq_canon: -1,
                rows_window: -1,
                window_hours: cli.window_hours,
                thresholds: Thresholds {
                    min_lr: cli.min_lr,
                    min_rng: cli.min_rng,
                    min_lane: cli.min_lane,
                    min_rows: cli.min_rows,
                },
                evidence_sql: EVIDENCE_SQL,
                anomalies: vec![format!("infra: {e:#}")],
            };
            if let Ok(j) = serde_json::to_string(&v) {
                println!("{j}");
            }
            // R5 fail-closed: infra error is treated as RED for the gardener
            ExitCode::from(2)
        }
    }
}

async fn run(cli: &Cli) -> Result<Verdict> {
    info!(
        "diversity gate starting: window={}h thresholds(LR≥{}, rng≥{}, lane≥{}, rows≥{})",
        cli.window_hours, cli.min_lr, cli.min_rng, cli.min_lane, cli.min_rows
    );

    let (client, conn) = tokio_postgres::connect(&cli.neon_url, NoTls)
        .await
        .context("postgres connect")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            warn!("postgres conn error: {e}");
        }
    });

    let row = client
        .query_one(EVIDENCE_SQL, &[&cli.window_hours.to_string()])
        .await
        .context("query diversity")?;

    let uniq_canon: i64 = row.get("uniq_canon");
    let uniq_lr: i64 = row.get("uniq_lr");
    let uniq_rng: i64 = row.get("uniq_rng");
    let uniq_lane: i64 = row.get("uniq_lane");
    let rows_window: i64 = row.get("rows_window");

    let mut anomalies = Vec::new();
    if rows_window < cli.min_rows {
        anomalies.push(format!(
            "rows_window={} below min_rows={} (writer might be stalled)",
            rows_window, cli.min_rows
        ));
    }
    if uniq_lr < cli.min_lr {
        anomalies.push(format!(
            "uniq_lr={} below min_lr={} (R7 violation: no LR ablation)",
            uniq_lr, cli.min_lr
        ));
    }
    if uniq_rng < cli.min_rng {
        anomalies.push(format!(
            "uniq_rng={} below min_rng={} (R7 violation: no seed ablation)",
            uniq_rng, cli.min_rng
        ));
    }
    if uniq_lane < cli.min_lane {
        anomalies.push(format!(
            "uniq_lane={} below min_lane={} (LANE registry mono-culture)",
            uniq_lane, cli.min_lane
        ));
    }

    let verdict = if anomalies.is_empty() {
        "green"
    } else if rows_window < cli.min_rows {
        "infra_error"
    } else {
        "red"
    };

    Ok(Verdict {
        verdict,
        uniq_lr,
        uniq_rng,
        uniq_lane,
        uniq_canon,
        rows_window,
        window_hours: cli.window_hours,
        thresholds: Thresholds {
            min_lr: cli.min_lr,
            min_rng: cli.min_rng,
            min_lane: cli.min_lane,
            min_rows: cli.min_rows,
        },
        evidence_sql: EVIDENCE_SQL,
        anomalies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: SQL string contains the diversity axes.
    #[test]
    fn evidence_sql_covers_all_axes() {
        assert!(EVIDENCE_SQL.contains("uniq_canon"));
        assert!(EVIDENCE_SQL.contains("uniq_lr"));
        assert!(EVIDENCE_SQL.contains("uniq_rng"));
        assert!(EVIDENCE_SQL.contains("uniq_lane"));
        assert!(EVIDENCE_SQL.contains("rows_window"));
    }

    /// R5: thresholds default to ≥3 LR, ≥3 rng, ≥1 lane (the IGLA-774 spec).
    #[test]
    fn default_thresholds_match_issue_774() {
        let cli = Cli::parse_from(["test", "--neon-url", "postgres://x"]);
        assert_eq!(cli.min_lr, 3);
        assert_eq!(cli.min_rng, 3);
        assert_eq!(cli.min_lane, 1);
    }
}

//! Per-tick loop driver. Pure orchestration: gathers a [`Context`],
//! calls `decide::decide`, applies the decisions (or skips under
//! dry-run), writes the projection to Neon.
//!
//! In PR-1 the I/O sides (`gather_*`, `apply_*`, `write_neon`) are
//! deliberately stubbed so the test surface is the orchestration
//! shape, not the live transport. PR-2 fills the stubs in.

use anyhow::Result;

use crate::decide::decide;
use crate::neon;
use crate::state::{Context, Decision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// First-3-ticks review window. Gardener calls `decide()` and
    /// records every decision under `dry_run_review=true`, but skips
    /// every mutation. Same code path as DryRun otherwise.
    Review,
    DryRun,
    Live,
}

/// Run one tick. Returns the list of decisions the gardener emitted
/// so the caller can write them to Neon and post a digest.
pub async fn loop_once(ctx: &Context, mode: RunMode) -> Result<Vec<Decision>> {
    let decisions = decide(ctx);

    match mode {
        RunMode::Review | RunMode::DryRun => {
            for d in &decisions {
                let (action, lane, seed) = neon::projection(d);
                tracing::info!(
                    %action,
                    ?lane,
                    ?seed,
                    "gardener[{:?}] would apply: {:?}",
                    mode,
                    d
                );
            }
        }
        RunMode::Live => {
            // Wired in PR-2: dispatch each Decision variant to the
            // corresponding tri-railway-core mutation.
            for d in &decisions {
                let (action, lane, seed) = neon::projection(d);
                tracing::warn!(
                    %action,
                    ?lane,
                    ?seed,
                    "gardener[Live]: apply path not yet implemented; recording dry-run only"
                );
            }
        }
    }

    Ok(decisions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        BpbLatest, BpbSample, FleetSnapshot, LaneSpec, PlateauHistory, Queue, RungWindow,
    };
    use chrono::{TimeZone, Utc};

    fn ctx_with_one_cull() -> Context {
        let now = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let mut bpb = BpbLatest::default();
        bpb.by_seed.insert(
            210,
            BpbSample {
                seed: 210,
                step: 9000,
                bpb: 2.45,
                last_reported_at: now,
            },
        );
        Context {
            now,
            window: RungWindow::Rung1To2,
            fleet: FleetSnapshot::default(),
            bpb,
            lanes: vec![LaneSpec {
                lane: "L1".into(),
                account: "acc1".into(),
                seeds: vec![210],
            }],
            queue: Queue { entries: vec![] },
            cleared_blockers: vec![],
            plateau: PlateauHistory::default(),
            free_slots: Default::default(),
            disabled: false,
        }
    }

    #[tokio::test]
    async fn dry_run_emits_decisions_but_no_mutations() {
        let ctx = ctx_with_one_cull();
        let out = loop_once(&ctx, RunMode::DryRun).await.unwrap();
        // Decision is computed; mutation path is not executed.
        assert!(out.iter().any(|d| matches!(d, Decision::CullSeed { .. })));
    }

    #[tokio::test]
    async fn review_mode_is_decisions_only() {
        let ctx = ctx_with_one_cull();
        let out = loop_once(&ctx, RunMode::Review).await.unwrap();
        assert!(!out.is_empty());
    }
}

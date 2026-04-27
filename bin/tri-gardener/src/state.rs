//! Pure state types for the gardener loop.
//!
//! `Context` is the input to `decide::decide`; `Decision` is the
//! output. Both are intentionally trivial to construct in tests
//! (no I/O, no clocks, no env reads).

// PR-1 ships the decision-table core; some Context fields (e.g. `now`,
// `expected_delta_bpb`) are wired by PR-2's loop driver. Allow them
// dead during PR-1 review.
#![allow(dead_code)]

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Race deadline anchor used to compute `t_minus`.
pub const GATE2_DEADLINE: &str = "2026-04-30T23:59:00Z";

/// Snapshot of every trainer service the gardener can see.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetSnapshot {
    pub services: Vec<ServiceObs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceObs {
    pub account: String,
    pub project_id: String,
    pub service_id: String,
    pub name: String,
    pub seed: Option<u32>,
    pub last_deploy_status: Option<String>,
    pub observed_at: DateTime<Utc>,
}

/// Latest BPB reading per (lane, seed). The gardener reads this from
/// the `bpb_samples` Neon table; tests synthesise it directly.
#[derive(Debug, Clone, Default)]
pub struct BpbLatest {
    pub by_seed: BTreeMap<u32, BpbSample>,
}

#[derive(Debug, Clone, Copy)]
pub struct BpbSample {
    pub seed: u32,
    pub step: u32,
    pub bpb: f64,
    pub last_reported_at: DateTime<Utc>,
}

/// Lane configuration as loaded from the plan-21 manifest. Only the
/// fields gardener actually needs for decisions are tracked.
#[derive(Debug, Clone)]
pub struct LaneSpec {
    pub lane: String,
    pub account: String,
    pub seeds: Vec<u32>,
}

/// Queue entry from `queue.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct QueueEntry {
    pub priority: u32,
    pub lane: String,
    pub account: String,
    pub seeds: Vec<u32>,
    #[serde(default)]
    pub expected_delta_bpb: Option<f64>,
    #[serde(default)]
    pub blocked_on: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Queue {
    #[serde(rename = "queue", default)]
    pub entries: Vec<QueueEntry>,
}

impl Queue {
    /// Highest-priority entry whose `blocked_on` is fully covered by
    /// `cleared`.
    pub fn next_unblocked<'a>(&'a self, cleared: &[String]) -> Option<&'a QueueEntry> {
        let mut sorted: Vec<&QueueEntry> = self.entries.iter().collect();
        sorted.sort_by_key(|e| e.priority);
        sorted
            .into_iter()
            .find(|e| e.blocked_on.iter().all(|b| cleared.iter().any(|c| c == b)))
    }
}

/// Per-lane plateau evaluation history. The gardener stores the last
/// 5 best-BPB readings per lane and triggers when they collapse to a
/// 0.005 band at step ≥ 50_000.
#[derive(Debug, Clone, Default)]
pub struct PlateauHistory {
    pub recent_best_bpb: BTreeMap<String, Vec<(u32, f64)>>, // lane -> [(step, bpb)]
}

/// Time-since-deadline derived once per tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RungWindow {
    PreRung1,     // T < +12h, redeploy missing only
    Rung1To2,     // +12h..+18h, cull > 2.30
    Rung2To3,     // +18h..+28h, cull > 2.20
    Rung3ToFinal, // +28h..+50h, cull > 2.05
    Final,        // >= +50h, promote survivors
    PostGate2,    // >= +54h marathon mode
}

impl RungWindow {
    /// Compute the rung window for `now` against the Gate-2 deadline.
    /// Race start anchored at 2026-04-27T18:00:00Z (T+0).
    pub fn from_now(now: DateTime<Utc>) -> Self {
        let race_start: DateTime<Utc> = "2026-04-27T18:00:00Z".parse().expect("race_start parses");
        let t_plus = now.signed_duration_since(race_start);
        let h = t_plus.num_hours();
        match h {
            x if x < 12 => RungWindow::PreRung1,
            12..=17 => RungWindow::Rung1To2,
            18..=27 => RungWindow::Rung2To3,
            28..=49 => RungWindow::Rung3ToFinal,
            50..=53 => RungWindow::Final,
            _ => RungWindow::PostGate2,
        }
    }
}

/// Everything `decide::decide` needs.
#[derive(Debug, Clone)]
pub struct Context {
    pub now: DateTime<Utc>,
    pub window: RungWindow,
    pub fleet: FleetSnapshot,
    pub bpb: BpbLatest,
    pub lanes: Vec<LaneSpec>,
    pub queue: Queue,
    pub cleared_blockers: Vec<String>,
    pub plateau: PlateauHistory,
    /// Free deploy-quota slots per account name (e.g. `"acc0" → 19`).
    pub free_slots: BTreeMap<String, u32>,
    /// Operator override: hard-disable mutations regardless of dry_run.
    pub disabled: bool,
}

/// Output of the decision pass. Each variant is a single mutation the
/// loop will execute (or log under dry-run).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Decision {
    Noop {
        reason: String,
    },
    RedeployMissing {
        lane: String,
        seed: u32,
        reason: String,
    },
    CullSeed {
        lane: String,
        seed: u32,
        bpb: BpbStr,
        threshold: BpbStr,
    },
    PromoteSurvivor {
        lane: String,
        seed: u32,
        bpb: BpbStr,
        promote_to_lane: String,
    },
    DeployQueueHead {
        lane: String,
        account: String,
        seeds: Vec<u32>,
    },
    PlateauAlert {
        lane: String,
        last_5: Vec<(u32, BpbStr)>,
        proposed_next: Option<String>,
    },
    HonestNotYet {
        best_bpb: BpbStr,
        target: BpbStr,
    },
}

/// `f64` BPB rendered to a 4-decimal string so `Eq`/`Hash` work for
/// snapshot tests without dragging in a float-eq dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BpbStr(pub String);

impl BpbStr {
    pub fn new(v: f64) -> Self {
        BpbStr(format!("{v:.4}"))
    }
}

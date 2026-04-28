//! ADR-0081 strategic-decision tick.
//!
//! Reads `experiment_queue` + `bpb_samples` + `workers` snapshot from
//! Neon, runs pure decision logic over it, and writes back:
//!
//! - `INSERT INTO experiment_queue` for new experiments suggested by
//!   the heuristic (champion mirrors, gap-filling configs)
//! - `UPDATE experiment_queue SET status='pending', worker_id=NULL`
//!   for stale claims (worker heartbeat older than 2 minutes)
//! - `UPDATE experiment_queue SET priority=...` for re-prioritisation
//!   based on the live leaderboard
//! - `INSERT INTO gardener_decisions` audit row per action taken
//!
//! Pure logic is kept in `decisions_for_snapshot` so it's testable
//! without a database. The `apply_decisions` function turns those
//! decisions into Neon writes inside one explicit transaction.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Single experiment row from Neon (subset relevant to strategy).
#[derive(Debug, Clone, PartialEq)]
pub struct ExpRow {
    pub id: i64,
    pub canon_name: String,
    pub seed: i32,
    pub account: String,
    pub status: String,
    pub priority: i32,
    pub claimed_at: Option<DateTime<Utc>>,
    pub final_bpb: Option<f64>,
}

/// Single worker heartbeat row.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerRow {
    pub id: uuid::Uuid,
    pub last_heartbeat: DateTime<Utc>,
    pub current_exp_id: Option<i64>,
}

/// Snapshot fed into the pure decision function.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub now: DateTime<Utc>,
    pub experiments: Vec<ExpRow>,
    pub workers: Vec<WorkerRow>,
    /// Best (lowest) BPB observed per `(canon, seed)` from `bpb_samples`.
    pub best_bpb_by_canon_seed: Vec<(String, i32, f64)>,
    /// Stale-claim threshold (default 2 min). Workers with
    /// `last_heartbeat < now - this` are considered dead.
    pub stale_claim_threshold: Duration,
    /// Gate-2 BPB target (default 1.85).
    pub gate2_target_bpb: f64,
}

/// Strategic decision the gardener wants to make.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Decision {
    /// No work to do this tick.
    Noop { reason: String },
    /// Reset stale claims (worker died) back to `pending`.
    ResetStaleClaim { exp_ids: Vec<i64>, reason: String },
    /// Increase priority of an existing experiment.
    PriorityBoost {
        exp_id: i64,
        new_priority: i32,
        reason: String,
    },
    /// Spawn a mirror of an existing champion at a new seed.
    SpawnMirror {
        parent_canon: String,
        new_seed: i32,
        account: String,
        reason: String,
    },
    /// Enqueue a fresh experiment.
    Enqueue {
        canon_name: String,
        seed: i32,
        account: String,
        priority: i32,
        steps_budget: i32,
        reason: String,
    },
}

impl Decision {
    pub fn action_label(&self) -> &'static str {
        match self {
            Decision::Noop { .. } => "noop",
            Decision::ResetStaleClaim { .. } => "reset_stale_claim",
            Decision::PriorityBoost { .. } => "priority_boost",
            Decision::SpawnMirror { .. } => "spawn_mirror",
            Decision::Enqueue { .. } => "enqueue",
        }
    }
}

/// Pure decision over a snapshot.
///
/// Order of operations (each step is independent and idempotent):
/// 1. **Stale-claim recovery** — find rows in `claimed`/`running` that
///    belong to workers whose heartbeat is older than the threshold.
///    Reset to `pending` so another worker can pick them up.
/// 2. **Champion-mirror spawning** — for any `done` experiment whose
///    `final_bpb < gate2_target`, ensure 3 sibling seeds exist
///    `(parent_canon, seed_a/b/c)`. Missing ones get enqueued.
/// 3. **Priority boost** — running experiments whose latest BPB sample
///    beats the current leader by ≥ 0.05 get `priority=0`.
/// 4. If nothing else to do, emit a `Noop`.
pub fn decisions_for_snapshot(snap: &Snapshot) -> Vec<Decision> {
    let mut out: Vec<Decision> = Vec::new();

    // 1. Stale-claim recovery.
    let stale_cutoff = snap.now - snap.stale_claim_threshold;
    let live_workers: std::collections::HashSet<uuid::Uuid> = snap
        .workers
        .iter()
        .filter(|w| w.last_heartbeat >= stale_cutoff)
        .map(|w| w.id)
        .collect();
    let stale_exp_ids: Vec<i64> = snap
        .experiments
        .iter()
        .filter(|e| {
            (e.status == "claimed" || e.status == "running")
                && match e.claimed_at {
                    Some(t) => t < stale_cutoff,
                    None => false,
                }
                && !snap
                    .workers
                    .iter()
                    .any(|w| w.current_exp_id == Some(e.id) && live_workers.contains(&w.id))
        })
        .map(|e| e.id)
        .collect();
    if !stale_exp_ids.is_empty() {
        out.push(Decision::ResetStaleClaim {
            exp_ids: stale_exp_ids,
            reason: format!(
                "{} experiments held by workers with no heartbeat for >= {}s",
                out.len(),
                snap.stale_claim_threshold.num_seconds()
            ),
        });
    }

    // 2. Champion-mirror spawning.
    let champions: Vec<&ExpRow> = snap
        .experiments
        .iter()
        .filter(|e| {
            e.status == "done"
                && e.final_bpb
                    .map(|b| b < snap.gate2_target_bpb)
                    .unwrap_or(false)
        })
        .collect();
    for ch in &champions {
        let parent_canon = strip_seed_suffix(&ch.canon_name);
        let existing_seeds: std::collections::BTreeSet<i32> = snap
            .experiments
            .iter()
            .filter(|e| strip_seed_suffix(&e.canon_name) == parent_canon)
            .map(|e| e.seed)
            .collect();
        for seed in [ch.seed, ch.seed + 1, ch.seed + 2] {
            if !existing_seeds.contains(&seed) {
                out.push(Decision::SpawnMirror {
                    parent_canon: parent_canon.clone(),
                    new_seed: seed,
                    account: ch.account.clone(),
                    reason: format!(
                        "champion {} BPB={:.4} < gate2={:.2} — quorum mirror seed={}",
                        ch.canon_name,
                        ch.final_bpb.unwrap_or(0.0),
                        snap.gate2_target_bpb,
                        seed
                    ),
                });
            }
        }
    }

    // 3. Priority boost — running rows whose own best BPB beats the
    //    *next-best other* row by ≥ 0.05 should be top of the queue
    //    so anyone reclaiming knows to go for it first.
    for e in &snap.experiments {
        if e.status != "running" || e.priority == 0 {
            continue;
        }
        let our_best = snap
            .best_bpb_by_canon_seed
            .iter()
            .find(|(c, s, _)| *c == e.canon_name && *s == e.seed)
            .map(|(_, _, b)| *b);
        let Some(our_best) = our_best else { continue };
        let other_leader = snap
            .best_bpb_by_canon_seed
            .iter()
            .filter(|(c, s, _)| !(*c == e.canon_name && *s == e.seed))
            .map(|(_, _, b)| *b)
            .fold(f64::INFINITY, f64::min);
        if other_leader.is_finite() && our_best + 0.05 < other_leader {
            out.push(Decision::PriorityBoost {
                exp_id: e.id,
                new_priority: 0,
                reason: format!(
                    "best={:.4} beats next-best other={:.4} by >= 0.05",
                    our_best, other_leader
                ),
            });
        }
    }

    if out.is_empty() {
        out.push(Decision::Noop {
            reason: "nothing to do this tick".to_string(),
        });
    }
    out
}

/// Strip a `-rng<N>` or `-seed<N>` suffix to recover the parent
/// canonical name. Used to find mirror siblings.
fn strip_seed_suffix(canon: &str) -> String {
    if let Some(idx) = canon.rfind("-rng") {
        return canon[..idx].to_string();
    }
    if let Some(idx) = canon.rfind("-seed") {
        return canon[..idx].to_string();
    }
    canon.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap()
    }

    fn snap_base() -> Snapshot {
        Snapshot {
            now: now(),
            experiments: vec![],
            workers: vec![],
            best_bpb_by_canon_seed: vec![],
            stale_claim_threshold: Duration::minutes(2),
            gate2_target_bpb: 1.85,
        }
    }

    #[test]
    fn empty_snapshot_yields_noop() {
        let v = decisions_for_snapshot(&snap_base());
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0], Decision::Noop { .. }));
    }

    #[test]
    fn stale_claim_is_recovered() {
        let mut s = snap_base();
        s.experiments.push(ExpRow {
            id: 100,
            canon_name: "IGLA-X-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            status: "claimed".into(),
            priority: 50,
            claimed_at: Some(now() - Duration::minutes(5)),
            final_bpb: None,
        });
        s.workers.push(WorkerRow {
            id: Uuid::nil(),
            last_heartbeat: now() - Duration::minutes(10),
            current_exp_id: Some(100),
        });
        let v = decisions_for_snapshot(&s);
        assert!(v.iter().any(|d| matches!(d, Decision::ResetStaleClaim { exp_ids, .. } if exp_ids.contains(&100))));
    }

    #[test]
    fn live_worker_keeps_its_claim() {
        let mut s = snap_base();
        s.experiments.push(ExpRow {
            id: 200,
            canon_name: "IGLA-X-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            status: "claimed".into(),
            priority: 50,
            claimed_at: Some(now() - Duration::minutes(3)),
            final_bpb: None,
        });
        s.workers.push(WorkerRow {
            id: Uuid::nil(),
            last_heartbeat: now() - Duration::seconds(30),
            current_exp_id: Some(200),
        });
        let v = decisions_for_snapshot(&s);
        assert!(!v.iter().any(|d| matches!(d, Decision::ResetStaleClaim { .. })));
    }

    #[test]
    fn champion_spawns_three_mirrors() {
        // Use gate2=1.90 so train_v2 BPB=1.8921 qualifies as champion.
        let mut s = snap_base();
        s.gate2_target_bpb = 1.90;
        s.experiments.push(ExpRow {
            id: 1,
            canon_name: "IGLA-TRAIN_V2-FP32-E0042-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            status: "done".into(),
            priority: 0,
            claimed_at: None,
            final_bpb: Some(1.8921),
        });
        let v = decisions_for_snapshot(&s);
        let mirrors: Vec<_> = v
            .iter()
            .filter_map(|d| match d {
                Decision::SpawnMirror { new_seed, parent_canon, .. } => {
                    Some((*new_seed, parent_canon.clone()))
                }
                _ => None,
            })
            .collect();
        // Champion already has seed 42 in queue; only 43 and 44 should be spawned.
        assert_eq!(mirrors.len(), 2);
        let parent = "IGLA-TRAIN_V2-FP32-E0042";
        assert!(mirrors.iter().any(|(s, p)| *s == 43 && p == parent));
        assert!(mirrors.iter().any(|(s, p)| *s == 44 && p == parent));
    }

    #[test]
    fn champion_above_gate_does_not_spawn_mirrors() {
        let mut s = snap_base();
        s.experiments.push(ExpRow {
            id: 1,
            canon_name: "IGLA-HYBRID-FP32-E0001-rng43".into(),
            seed: 43,
            account: "acc1".into(),
            status: "done".into(),
            priority: 0,
            claimed_at: None,
            final_bpb: Some(2.1919), // > gate2_target=1.85
        });
        let v = decisions_for_snapshot(&s);
        assert!(!v.iter().any(|d| matches!(d, Decision::SpawnMirror { .. })));
    }

    #[test]
    fn priority_boost_when_running_beats_leader_by_threshold() {
        let mut s = snap_base();
        s.experiments.push(ExpRow {
            id: 7,
            canon_name: "IGLA-NEW-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            status: "running".into(),
            priority: 50,
            claimed_at: Some(now()),
            final_bpb: None,
        });
        s.best_bpb_by_canon_seed
            .push(("IGLA-NEW-rng42".into(), 42, 1.70));
        s.best_bpb_by_canon_seed
            .push(("IGLA-OLD-rng99".into(), 99, 1.80)); // global leader
        let v = decisions_for_snapshot(&s);
        assert!(v.iter().any(|d| matches!(d, Decision::PriorityBoost { exp_id: 7, .. })));
    }

    #[test]
    fn priority_boost_does_not_fire_when_already_top() {
        let mut s = snap_base();
        s.experiments.push(ExpRow {
            id: 7,
            canon_name: "IGLA-NEW-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            status: "running".into(),
            priority: 0, // already top
            claimed_at: Some(now()),
            final_bpb: None,
        });
        s.best_bpb_by_canon_seed
            .push(("IGLA-NEW-rng42".into(), 42, 1.70));
        s.best_bpb_by_canon_seed
            .push(("IGLA-OLD-rng99".into(), 99, 1.80));
        let v = decisions_for_snapshot(&s);
        assert!(!v.iter().any(|d| matches!(d, Decision::PriorityBoost { .. })));
    }

    #[test]
    fn strip_suffix_handles_rng_and_seed_and_neither() {
        assert_eq!(strip_seed_suffix("IGLA-X-rng42"), "IGLA-X");
        assert_eq!(strip_seed_suffix("IGLA-X-seed99"), "IGLA-X");
        assert_eq!(strip_seed_suffix("IGLA-X"), "IGLA-X");
    }

    #[test]
    fn decision_action_labels_match_ddl_check() {
        // Must match gardener_decisions.action CHECK constraint in
        // trios-railway-audit migrations.
        let cases = [
            Decision::Noop { reason: "x".into() },
            Decision::ResetStaleClaim { exp_ids: vec![1], reason: "x".into() },
            Decision::PriorityBoost { exp_id: 1, new_priority: 0, reason: "x".into() },
            Decision::SpawnMirror {
                parent_canon: "IGLA-X".into(),
                new_seed: 42,
                account: "acc1".into(),
                reason: "x".into(),
            },
            Decision::Enqueue {
                canon_name: "IGLA-Y-rng1".into(),
                seed: 1,
                account: "acc0".into(),
                priority: 50,
                steps_budget: 81000,
                reason: "x".into(),
            },
        ];
        for d in cases {
            assert!(matches!(
                d.action_label(),
                "noop" | "reset_stale_claim" | "priority_boost" | "spawn_mirror" | "enqueue"
            ));
        }
    }

    #[test]
    fn decisions_are_serde_round_trippable() {
        let d = Decision::Enqueue {
            canon_name: "IGLA-X-rng42".into(),
            seed: 42,
            account: "acc1".into(),
            priority: 10,
            steps_budget: 1000,
            reason: "boot strap".into(),
        };
        let s = serde_json::to_string(&d).unwrap();
        let back: Decision = serde_json::from_str(&s).unwrap();
        assert_eq!(d, back);
    }
}

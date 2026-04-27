//! Gardener state types and pure decision table.
//!
//! `decide()` is pure over `Context` — this is the unit-test surface.
//! All mutations (deploy, cull, promote) are handled by the caller.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BpbSample {
    pub seed: u32,
    pub lane: String,
    pub bpb: f64,
    pub step: u64,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub priority: u32,
    pub lane: String,
    pub account: String,
    pub seeds: Vec<u32>,
    pub expected_delta_bpb: f64,
    pub blocked_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetService {
    pub name: String,
    pub service_id: String,
    pub account: String,
    pub lane: String,
    pub seed: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Crashed,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub snapshot: Vec<FleetService>,
    pub bpb_samples: Vec<BpbSample>,
    pub queue: Vec<QueueEntry>,
    pub cleared_blockers: Vec<String>,
    pub t_minus_hours: f64,
    pub target_bpb: f64,
    #[allow(dead_code)]
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Decision {
    Deploy {
        lane: String,
        account: String,
        seeds: Vec<u32>,
    },
    Cull {
        lane: String,
        seed: u32,
        bpb: f64,
        threshold: f64,
    },
    Promote {
        lane: String,
        seeds: Vec<u32>,
        best_bpb: f64,
    },
    Redeploy {
        service_id: String,
        reason: String,
    },
    PlateauAlert {
        lane: String,
        bpb: f64,
        proposal: String,
    },
    Noop {
        reason: String,
    },
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Decision::Deploy { lane, seeds, .. } => {
                write!(f, "DEPLOY lane={lane} seeds={seeds:?}")
            }
            Decision::Cull {
                lane, seed, bpb, ..
            } => {
                write!(f, "CULL lane={lane} seed={seed} bpb={bpb:.4}")
            }
            Decision::Promote {
                lane,
                seeds,
                best_bpb,
                ..
            } => {
                write!(f, "PROMOTE lane={lane} seeds={seeds:?} bpb={best_bpb:.4}")
            }
            Decision::Redeploy {
                service_id, reason, ..
            } => {
                write!(f, "REDEPLOY {service_id} ({reason})")
            }
            Decision::PlateauAlert { lane, bpb, .. } => {
                write!(f, "PLATEAU lane={lane} bpb={bpb:.4}")
            }
            Decision::Noop { reason } => {
                write!(f, "NOOP ({reason})")
            }
        }
    }
}

/// Pure decision table. Returns a list of actions to take.
#[allow(clippy::too_many_lines)]
pub fn decide(ctx: &Context) -> Vec<Decision> {
    let mut decisions = Vec::new();

    let lanes: Vec<&str> = ctx
        .bpb_samples
        .iter()
        .map(|s| s.lane.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    for lane in &lanes {
        let lane_seeds: Vec<&BpbSample> =
            ctx.bpb_samples.iter().filter(|s| s.lane == *lane).collect();

        let running_in_lane: Vec<&FleetService> = ctx
            .snapshot
            .iter()
            .filter(|s| s.lane == *lane && s.status == ServiceStatus::Running)
            .collect();

        if running_in_lane.len() < 3 && ctx.t_minus_hours < 12.0 {
            let missing_seeds: Vec<u32> = ctx
                .queue
                .iter()
                .filter(|q| q.lane == *lane)
                .flat_map(|q| q.seeds.iter().copied())
                .filter(|s| !running_in_lane.iter().any(|r| r.seed == *s))
                .collect();
            if !missing_seeds.is_empty() {
                decisions.push(Decision::Deploy {
                    lane: (*lane).to_string(),
                    account: running_in_lane
                        .first()
                        .map(|s| s.account.clone())
                        .unwrap_or_default(),
                    seeds: missing_seeds,
                });
            }
        }

        let threshold = cull_threshold(ctx.t_minus_hours);
        for sample in &lane_seeds {
            if sample.bpb > threshold {
                decisions.push(Decision::Cull {
                    lane: (*lane).to_string(),
                    seed: sample.seed,
                    bpb: sample.bpb,
                    threshold,
                });
            }
        }

        if ctx.t_minus_hours >= 50.0 {
            let survivors: Vec<&BpbSample> = lane_seeds
                .iter()
                .filter(|s| s.bpb <= ctx.target_bpb)
                .copied()
                .collect();
            if survivors.len() >= 2 {
                let best = survivors
                    .iter()
                    .map(|s| s.bpb)
                    .fold(f64::INFINITY, f64::min);
                let seeds: Vec<u32> = survivors.iter().map(|s| s.seed).collect();
                decisions.push(Decision::Promote {
                    lane: (*lane).to_string(),
                    seeds,
                    best_bpb: best,
                });
            }
        }

        if is_plateau(&lane_seeds) {
            let best_bpb = lane_seeds
                .iter()
                .map(|s| s.bpb)
                .fold(f64::INFINITY, f64::min);
            let proposal = ctx
                .queue
                .iter()
                .filter(|q| {
                    q.lane != *lane
                        && q.blocked_on
                            .iter()
                            .all(|b| ctx.cleared_blockers.contains(b))
                })
                .max_by_key(|q| q.priority)
                .map_or_else(
                    || "no unblocked queue entries".into(),
                    |q| format!("queue entry: lane={} seeds={:?}", q.lane, q.seeds),
                );
            decisions.push(Decision::PlateauAlert {
                lane: (*lane).to_string(),
                bpb: best_bpb,
                proposal,
            });
        }
    }

    for svc in &ctx.snapshot {
        if svc.status == ServiceStatus::Crashed {
            decisions.push(Decision::Redeploy {
                service_id: svc.service_id.clone(),
                reason: "crashed".into(),
            });
        }
    }

    if decisions.is_empty() {
        decisions.push(Decision::Noop {
            reason: "no triggers fired".into(),
        });
    }

    decisions
}

fn cull_threshold(t_minus: f64) -> f64 {
    if t_minus < 12.0 {
        f64::INFINITY
    } else if t_minus < 18.0 {
        2.30
    } else if t_minus < 28.0 {
        2.20
    } else if t_minus < 50.0 {
        2.05
    } else {
        f64::INFINITY
    }
}

fn is_plateau(samples: &[&BpbSample]) -> bool {
    if samples.len() < 5 {
        return false;
    }
    let best_bpbs: Vec<f64> = {
        let mut s = samples.to_vec();
        s.sort_by(|a, b| b.ts.cmp(&a.ts));
        s.iter().take(5).map(|s| s.bpb).collect()
    };
    let max_bpb = best_bpbs
        .iter()
        .fold(f64::NEG_INFINITY, |acc, &v| f64::max(acc, v));
    let min_bpb = best_bpbs
        .iter()
        .fold(f64::INFINITY, |acc, &v| f64::min(acc, v));
    let min_step = samples.iter().map(|s| s.step).min().unwrap_or(0);
    max_bpb - min_bpb < 0.005 && min_step >= 50_000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(t_minus: f64, samples: Vec<BpbSample>) -> Context {
        Context {
            snapshot: Vec::new(),
            bpb_samples: samples,
            queue: Vec::new(),
            cleared_blockers: Vec::new(),
            t_minus_hours: t_minus,
            target_bpb: 1.85,
            now: Utc::now(),
        }
    }

    fn sample(seed: u32, lane: &str, bpb: f64, step: u64) -> BpbSample {
        BpbSample {
            seed,
            lane: (*lane).to_string(),
            bpb,
            step,
            ts: Utc::now(),
        }
    }

    #[test]
    fn noop_when_nothing_to_do() {
        let ctx = make_ctx(5.0, vec![sample(43, "baseline", 2.18, 81_000)]);
        let decisions = decide(&ctx);
        assert!(decisions.iter().any(|d| matches!(d, Decision::Noop { .. })));
    }

    #[test]
    fn cull_high_bpb_after_12h() {
        let ctx = make_ctx(15.0, vec![sample(100, "baseline", 2.50, 27_000)]);
        let decisions = decide(&ctx);
        assert!(decisions
            .iter()
            .any(|d| matches!(d, Decision::Cull { seed: 100, .. })));
    }

    #[test]
    fn no_cull_before_12h() {
        let ctx = make_ctx(5.0, vec![sample(100, "baseline", 2.80, 10_000)]);
        let decisions = decide(&ctx);
        assert!(!decisions.iter().any(|d| matches!(d, Decision::Cull { .. })));
    }

    #[test]
    fn promote_after_50h_with_survivors() {
        let ctx = make_ctx(
            55.0,
            vec![
                sample(43, "baseline", 1.80, 81_000),
                sample(42, "baseline", 1.82, 81_000),
            ],
        );
        let decisions = decide(&ctx);
        assert!(decisions
            .iter()
            .any(|d| matches!(d, Decision::Promote { .. })));
    }

    #[test]
    fn crashed_service_gets_redeployed() {
        let ctx = Context {
            snapshot: vec![FleetService {
                name: "trios-train-seed-43".into(),
                service_id: "svc-1".into(),
                account: "acc1".into(),
                lane: "baseline".into(),
                seed: 43,
                status: ServiceStatus::Crashed,
            }],
            bpb_samples: vec![sample(43, "baseline", 2.18, 81_000)],
            queue: Vec::new(),
            cleared_blockers: Vec::new(),
            t_minus_hours: 5.0,
            target_bpb: 1.85,
            now: Utc::now(),
        };
        let decisions = decide(&ctx);
        assert!(decisions
            .iter()
            .any(|d| matches!(d, Decision::Redeploy { service_id, .. } if service_id == "svc-1")));
    }

    #[test]
    fn cull_threshold_schedule() {
        assert!(!cull_threshold(5.0).is_finite());
        assert!((cull_threshold(15.0) - 2.30).abs() < f64::EPSILON);
        assert!((cull_threshold(25.0) - 2.20).abs() < f64::EPSILON);
        assert!((cull_threshold(40.0) - 2.05).abs() < f64::EPSILON);
        assert!(!cull_threshold(60.0).is_finite());
    }

    #[test]
    fn plateau_detected_when_stable() {
        let samples: Vec<BpbSample> = (0..5)
            .map(|i| sample(43, "baseline", 2.18 + f64::from(i) * 0.0005, 60_000))
            .collect();
        assert!(is_plateau(&samples.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn no_plateau_when_still_improving() {
        let samples: Vec<BpbSample> = (0..5)
            .map(|i| sample(43, "baseline", 2.50 - f64::from(i) * 0.05, 60_000))
            .collect();
        assert!(!is_plateau(&samples.iter().collect::<Vec<_>>()));
    }

    #[test]
    fn no_plateau_below_min_steps() {
        let samples: Vec<BpbSample> = (0..5)
            .map(|_| sample(43, "baseline", 2.18, 10_000))
            .collect();
        assert!(!is_plateau(&samples.iter().collect::<Vec<_>>()));
    }
}

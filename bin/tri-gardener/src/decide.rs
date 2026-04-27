//! Pure decision table — no I/O, no clocks, no env reads.
//!
//! Take a [`Context`], return a `Vec<Decision>`. Every test is one
//! `Context` literal in, expected `Vec<Decision>` out. Wall-clock
//! `now` is owned by the caller (the loop driver) and baked into
//! `Context` before calling `decide`.

#![allow(
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::cast_possible_truncation,
    clippy::doc_markdown
)]

use crate::state::{BpbSample, BpbStr, Context, Decision, RungWindow};

/// BPB upper bound at which a seed survives the named rung. Looser
/// than the issue text on purpose: we stay one notch ahead of the
/// audit watchdog so culls happen before the watchdog raises drift.
pub fn cull_threshold(window: RungWindow) -> Option<f64> {
    match window {
        RungWindow::PreRung1 => None,
        RungWindow::Rung1To2 => Some(2.30),
        RungWindow::Rung2To3 => Some(2.20),
        RungWindow::Rung3ToFinal => Some(2.05),
        RungWindow::Final | RungWindow::PostGate2 => None,
    }
}

/// Plateau parameters per the spec in #49.
const PLATEAU_WINDOW: usize = 5;
const PLATEAU_BPB_BAND: f64 = 0.005;
const PLATEAU_MIN_STEP: u32 = 50_000;

/// Top-level decision pass. Order in the returned vec is the order
/// in which mutations will be applied.
pub fn decide(ctx: &Context) -> Vec<Decision> {
    if ctx.disabled {
        return vec![Decision::Noop {
            reason: "GARDENER_DISABLED=true".into(),
        }];
    }

    let mut out = Vec::new();

    // 1. PreRung1: redeploy missing seeds (idempotent at apply time).
    if matches!(ctx.window, RungWindow::PreRung1) {
        for lane in &ctx.lanes {
            for seed in &lane.seeds {
                if !fleet_has_seed(&ctx.fleet, *seed) {
                    out.push(Decision::RedeployMissing {
                        lane: lane.lane.clone(),
                        seed: *seed,
                        reason: "service missing from fleet snapshot".into(),
                    });
                }
            }
        }
    }

    // 2. Cull underperformers per current rung threshold.
    if let Some(thr) = cull_threshold(ctx.window) {
        for lane in &ctx.lanes {
            for seed in &lane.seeds {
                if let Some(s) = ctx.bpb.by_seed.get(seed) {
                    if s.bpb > thr {
                        out.push(Decision::CullSeed {
                            lane: lane.lane.clone(),
                            seed: *seed,
                            bpb: BpbStr::new(s.bpb),
                            threshold: BpbStr::new(thr),
                        });
                    }
                }
            }
        }
    }

    // 3. Final / PostGate2 promotion: any lane with ≥ 2 survivors
    //    under 1.85 promotes its top survivors to phase3 replicas.
    if matches!(ctx.window, RungWindow::Final | RungWindow::PostGate2) {
        for lane in &ctx.lanes {
            let mut survivors: Vec<BpbSample> = lane
                .seeds
                .iter()
                .filter_map(|s| ctx.bpb.by_seed.get(s).copied())
                .filter(|s| s.bpb < 1.85)
                .collect();
            // Best (lowest) first.
            survivors.sort_by(|a, b| {
                a.bpb
                    .partial_cmp(&b.bpb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if survivors.len() >= 2 {
                for s in survivors.iter().take(3) {
                    out.push(Decision::PromoteSurvivor {
                        lane: lane.lane.clone(),
                        seed: s.seed,
                        bpb: BpbStr::new(s.bpb),
                        promote_to_lane: "phase3_backup_replicas".into(),
                    });
                }
            }
        }
    }

    // 4. Plateau detector — orthogonal to rung windows.
    for (lane, hist) in &ctx.plateau.recent_best_bpb {
        if hist.len() < PLATEAU_WINDOW {
            continue;
        }
        let last: &[(u32, f64)] = &hist[hist.len() - PLATEAU_WINDOW..];
        let min = last.iter().map(|(_, b)| *b).fold(f64::INFINITY, f64::min);
        let max = last
            .iter()
            .map(|(_, b)| *b)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_step = last.iter().map(|(s, _)| *s).min().unwrap_or(0);
        if max - min < PLATEAU_BPB_BAND && min_step >= PLATEAU_MIN_STEP {
            out.push(Decision::PlateauAlert {
                lane: lane.clone(),
                last_5: last.iter().map(|(s, b)| (*s, BpbStr::new(*b))).collect(),
                proposed_next: ctx
                    .queue
                    .next_unblocked(&ctx.cleared_blockers)
                    .map(|q| q.lane.clone()),
            });
        }
    }

    // 5. Queue head deploy if any account has free slots.
    if let Some(head) = ctx.queue.next_unblocked(&ctx.cleared_blockers) {
        let free = ctx.free_slots.get(&head.account).copied().unwrap_or(0);
        let need = head.seeds.len() as u32;
        if free >= need && !lane_already_running(ctx, &head.lane) {
            out.push(Decision::DeployQueueHead {
                lane: head.lane.clone(),
                account: head.account.clone(),
                seeds: head.seeds.clone(),
            });
        }
    }

    // 6. Honest-not-yet: PostGate2 with no lane meeting Gate-2.
    if matches!(ctx.window, RungWindow::PostGate2) {
        let any_pass = ctx.lanes.iter().any(|lane| {
            lane.seeds
                .iter()
                .filter_map(|s| ctx.bpb.by_seed.get(s))
                .filter(|s| s.bpb < 1.85)
                .count()
                >= 3
        });
        if !any_pass {
            let best = ctx
                .bpb
                .by_seed
                .values()
                .map(|s| s.bpb)
                .fold(f64::INFINITY, f64::min);
            out.push(Decision::HonestNotYet {
                best_bpb: BpbStr::new(best),
                target: BpbStr::new(1.85),
            });
        }
    }

    if out.is_empty() {
        out.push(Decision::Noop {
            reason: "no triggers in current window".into(),
        });
    }
    out
}

fn fleet_has_seed(fleet: &crate::state::FleetSnapshot, seed: u32) -> bool {
    fleet.services.iter().any(|s| s.seed == Some(seed))
}

fn lane_already_running(ctx: &Context, lane: &str) -> bool {
    ctx.lanes.iter().any(|l| l.lane == lane)
}

// ---------------------------------------------------------------------------
// Tests — every row of the decision table covered.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        BpbLatest, FleetSnapshot, LaneSpec, PlateauHistory, Queue, QueueEntry, ServiceObs,
    };
    use chrono::{TimeZone, Utc};

    fn now() -> chrono::DateTime<chrono::Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap()
    }

    fn empty_ctx(window: RungWindow) -> Context {
        Context {
            now: now(),
            window,
            fleet: FleetSnapshot::default(),
            bpb: BpbLatest::default(),
            lanes: Vec::new(),
            queue: Queue { entries: vec![] },
            cleared_blockers: vec![],
            plateau: PlateauHistory::default(),
            free_slots: Default::default(),
            disabled: false,
        }
    }

    fn lane(name: &str, seeds: &[u32]) -> LaneSpec {
        LaneSpec {
            lane: name.into(),
            account: "acc1".into(),
            seeds: seeds.to_vec(),
        }
    }

    fn bpb(seed: u32, step: u32, val: f64) -> BpbSample {
        BpbSample {
            seed,
            step,
            bpb: val,
            last_reported_at: now(),
        }
    }

    #[test]
    fn disabled_short_circuits() {
        let mut ctx = empty_ctx(RungWindow::Rung1To2);
        ctx.disabled = true;
        let d = decide(&ctx);
        assert!(matches!(&d[..], [Decision::Noop { .. }]));
    }

    #[test]
    fn pre_rung1_redeploys_missing_seed() {
        let mut ctx = empty_ctx(RungWindow::PreRung1);
        ctx.lanes.push(lane("L4_ema_n10", &[240, 241, 242]));
        ctx.fleet.services.push(ServiceObs {
            account: "acc0".into(),
            project_id: "p".into(),
            service_id: "s".into(),
            name: "trios-train-seed-240-L4-ema-n10".into(),
            seed: Some(240),
            last_deploy_status: Some("SUCCESS".into()),
            observed_at: now(),
        });
        let d = decide(&ctx);
        // Two missing seeds (241, 242) → exactly 2 RedeployMissing decisions.
        let redeploys: Vec<_> = d
            .iter()
            .filter(|x| matches!(x, Decision::RedeployMissing { .. }))
            .collect();
        assert_eq!(redeploys.len(), 2);
    }

    #[test]
    fn rung1_to_2_culls_above_2_30() {
        let mut ctx = empty_ctx(RungWindow::Rung1To2);
        ctx.lanes.push(lane("L1", &[210, 211]));
        ctx.bpb.by_seed.insert(210, bpb(210, 9000, 2.45));
        ctx.bpb.by_seed.insert(211, bpb(211, 9000, 2.10));
        let d = decide(&ctx);
        let culled: Vec<_> = d
            .iter()
            .filter_map(|x| match x {
                Decision::CullSeed { seed, .. } => Some(*seed),
                _ => None,
            })
            .collect();
        assert_eq!(culled, vec![210]);
    }

    #[test]
    fn rung3_to_final_culls_above_2_05() {
        let mut ctx = empty_ctx(RungWindow::Rung3ToFinal);
        ctx.lanes.push(lane("L4-lite", &[250, 251, 252]));
        ctx.bpb.by_seed.insert(250, bpb(250, 27000, 2.10));
        ctx.bpb.by_seed.insert(251, bpb(251, 27000, 2.00));
        ctx.bpb.by_seed.insert(252, bpb(252, 27000, 1.95));
        let d = decide(&ctx);
        let culled: Vec<_> = d
            .iter()
            .filter_map(|x| match x {
                Decision::CullSeed { seed, .. } => Some(*seed),
                _ => None,
            })
            .collect();
        assert_eq!(culled, vec![250]);
    }

    #[test]
    fn final_promotes_survivors_after_rung3() {
        let mut ctx = empty_ctx(RungWindow::Final);
        ctx.lanes.push(lane("L1_attn_backward", &[210, 211, 212]));
        ctx.bpb.by_seed.insert(210, bpb(210, 81000, 1.80));
        ctx.bpb.by_seed.insert(211, bpb(211, 81000, 1.78));
        ctx.bpb.by_seed.insert(212, bpb(212, 81000, 1.92)); // above 1.85
        let d = decide(&ctx);
        let promoted: Vec<_> = d
            .iter()
            .filter_map(|x| match x {
                Decision::PromoteSurvivor {
                    seed,
                    promote_to_lane,
                    ..
                } => Some((*seed, promote_to_lane.clone())),
                _ => None,
            })
            .collect();
        // Two seeds < 1.85 → both promoted to phase3_backup_replicas.
        assert_eq!(promoted.len(), 2);
        for (_seed, target) in &promoted {
            assert_eq!(target, "phase3_backup_replicas");
        }
    }

    #[test]
    fn plateau_alert_when_5_ticks_within_band_and_above_50k() {
        let mut ctx = empty_ctx(RungWindow::Rung3ToFinal);
        ctx.lanes.push(lane("L4_capacity_h2000", &[240]));
        ctx.plateau
            .recent_best_bpb
            .insert("L4_capacity_h2000".into(), {
                vec![
                    (51_000, 2.190),
                    (54_000, 2.191),
                    (57_000, 2.189),
                    (60_000, 2.192),
                    (63_000, 2.190),
                ]
            });
        let d = decide(&ctx);
        assert!(d.iter().any(|x| matches!(x, Decision::PlateauAlert { .. })));
    }

    #[test]
    fn plateau_skipped_below_min_step() {
        let mut ctx = empty_ctx(RungWindow::Rung3ToFinal);
        ctx.lanes.push(lane("L4", &[240]));
        ctx.plateau.recent_best_bpb.insert("L4".into(), {
            vec![
                (10_000, 2.30),
                (12_000, 2.30),
                (14_000, 2.30),
                (16_000, 2.30),
                (18_000, 2.30),
            ]
        });
        let d = decide(&ctx);
        assert!(!d.iter().any(|x| matches!(x, Decision::PlateauAlert { .. })));
    }

    #[test]
    fn deploy_queue_head_when_blocker_cleared_and_slots_free() {
        let mut ctx = empty_ctx(RungWindow::Rung3ToFinal);
        ctx.queue = Queue {
            entries: vec![QueueEntry {
                priority: 1,
                lane: "L7_h1024_6L".into(),
                account: "acc0".into(),
                seeds: vec![600, 601, 602],
                expected_delta_bpb: Some(-0.30),
                blocked_on: vec!["trainer-igla:l1_attention_backward".into()],
            }],
        };
        ctx.cleared_blockers
            .push("trainer-igla:l1_attention_backward".into());
        ctx.free_slots.insert("acc0".into(), 19);
        let d = decide(&ctx);
        assert!(d.iter().any(|x| matches!(
            x,
            Decision::DeployQueueHead { lane, .. } if lane == "L7_h1024_6L"
        )));
    }

    #[test]
    fn deploy_queue_head_blocked_when_blocker_not_cleared() {
        let mut ctx = empty_ctx(RungWindow::Rung3ToFinal);
        ctx.queue = Queue {
            entries: vec![QueueEntry {
                priority: 1,
                lane: "L7_h1024_6L".into(),
                account: "acc0".into(),
                seeds: vec![600, 601, 602],
                expected_delta_bpb: Some(-0.30),
                blocked_on: vec!["trainer-igla:l1_attention_backward".into()],
            }],
        };
        ctx.free_slots.insert("acc0".into(), 19);
        // No cleared blockers.
        let d = decide(&ctx);
        assert!(!d
            .iter()
            .any(|x| matches!(x, Decision::DeployQueueHead { .. })));
    }

    #[test]
    fn post_gate2_emits_honest_not_yet_when_no_quorum() {
        let mut ctx = empty_ctx(RungWindow::PostGate2);
        ctx.lanes.push(lane("L1", &[210, 211, 212]));
        // All three above target.
        ctx.bpb.by_seed.insert(210, bpb(210, 81000, 2.10));
        ctx.bpb.by_seed.insert(211, bpb(211, 81000, 2.05));
        ctx.bpb.by_seed.insert(212, bpb(212, 81000, 1.95));
        let d = decide(&ctx);
        assert!(d.iter().any(|x| matches!(x, Decision::HonestNotYet { .. })));
    }

    #[test]
    fn rung_window_boundaries() {
        let race_start: chrono::DateTime<Utc> = "2026-04-27T18:00:00Z".parse().unwrap();
        assert_eq!(RungWindow::from_now(race_start), RungWindow::PreRung1);
        assert_eq!(
            RungWindow::from_now(race_start + chrono::Duration::hours(13)),
            RungWindow::Rung1To2
        );
        assert_eq!(
            RungWindow::from_now(race_start + chrono::Duration::hours(20)),
            RungWindow::Rung2To3
        );
        assert_eq!(
            RungWindow::from_now(race_start + chrono::Duration::hours(30)),
            RungWindow::Rung3ToFinal
        );
        assert_eq!(
            RungWindow::from_now(race_start + chrono::Duration::hours(52)),
            RungWindow::Final
        );
        assert_eq!(
            RungWindow::from_now(race_start + chrono::Duration::hours(60)),
            RungWindow::PostGate2
        );
    }
}

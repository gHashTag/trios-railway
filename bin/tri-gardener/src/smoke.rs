//! GATE-0 SMOKE RACE — pure-logic orchestrator.
//!
//! End-to-end pipe-cleaner for the 21-service mini-race that runs
//! before any 12-hour production training. Goal: prove every tube
//! works (deploy → train loop → BPB stdout → telemetry ingest →
//! ledger → leaderboard) before committing real compute.
//!
//! All planning logic in this module is pure: it produces a list of
//! `SmokePlanEntry` values and runs the **14 acceptance criteria** as
//! Rust functions over the plan. The live infrastructure pieces
//! (Railway batch deploy, Neon polling, watchdog kill) are owned by
//! the caller (`bin/tri-gardener/src/main.rs::Cmd::SmokeRace`).
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · SMOKE BEFORE FIRE`.

use std::collections::BTreeMap;

use crate::canon::{
    assert_kill_before_spin, assert_smoke_seed_range, champion_lock_reason, is_smoke,
    CanonError, IglaCanon, ModelType, NumberFormat, SMOKE_SEED_RANGE, SMOKE_TAG_MARKER,
};

/// One row of the smoke plan: a canonical name + the rng/exp_id pair
/// the orchestrator will deploy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokePlanEntry {
    pub canon: IglaCanon,
    pub experiment_tag: &'static str, // base tag, e.g. "WSD" / "BS8" / ...
    pub max_steps: u32,
    pub eval_every: u32,
    pub bpb_emit_every: u32,
    pub wallclock_minutes: u32,
}

/// Smoke plan = the full 21-row schedule.
#[derive(Debug, Clone)]
pub struct SmokePlan {
    pub entries: Vec<SmokePlanEntry>,
    pub max_steps: u32,
    pub eval_every: u32,
    pub bpb_emit_every: u32,
    pub wallclock_minutes_per_seed: u32,
}

/// Default smoke parameters from the operator brief.
pub const SMOKE_MAX_STEPS: u32 = 500;
pub const SMOKE_EVAL_EVERY: u32 = 50;
pub const SMOKE_BPB_EMIT_EVERY: u32 = 10;
pub const SMOKE_WALLCLOCK_MIN: u32 = 30;
pub const SMOKE_FIRST_EXP_ID: u32 = 500;
pub const SMOKE_FIRST_RNG: u32 = 500;

/// Seven smoke experiment tags + their rng-seed offsets within the
/// reserved 500..600 window. Layout mirrors the operator's table:
/// each tag claims a contiguous block of three rng seeds.
pub const SMOKE_EXPERIMENTS: &[(ModelType, &str, [u32; 3])] = &[
    (ModelType::Hybrid, "WSD",           [500, 501, 502]),
    (ModelType::Hybrid, "BS8",           [510, 511, 512]),
    (ModelType::JepaT,  "GRADFIX_JEPAT", [520, 521, 522]),
    (ModelType::Nca,    "GRADFIX_NCA",   [530, 531, 532]),
    (ModelType::Hybrid, "EMA10",         [540, 541, 542]),
    (ModelType::Hybrid, "h512",          [550, 551, 552]),
    (ModelType::Hybrid, "h768",          [560, 561, 562]),
];

/// Build the canonical 21-row smoke plan. EXP_IDs start at
/// `first_exp_id` and increment by 1 for every entry, preserving
/// monotonicity across the whole plan (Tripwire #99-friendly).
pub fn build_default_plan(first_exp_id: u32) -> SmokePlan {
    let mut entries = Vec::with_capacity(21);
    let mut next_exp = first_exp_id;
    for (model, tag, rngs) in SMOKE_EXPERIMENTS {
        for rng in rngs {
            let canon = IglaCanon {
                model: *model,
                format: NumberFormat::Fp32,
                exp_id: Some(next_exp),
                tag: Some(format!("{tag}-{}", SMOKE_TAG_MARKER)),
                rng: Some(*rng),
                legacy_seed: None,
            };
            entries.push(SmokePlanEntry {
                canon,
                experiment_tag: tag,
                max_steps: SMOKE_MAX_STEPS,
                eval_every: SMOKE_EVAL_EVERY,
                bpb_emit_every: SMOKE_BPB_EMIT_EVERY,
                wallclock_minutes: SMOKE_WALLCLOCK_MIN,
            });
            next_exp += 1;
        }
    }
    SmokePlan {
        entries,
        max_steps: SMOKE_MAX_STEPS,
        eval_every: SMOKE_EVAL_EVERY,
        bpb_emit_every: SMOKE_BPB_EMIT_EVERY,
        wallclock_minutes_per_seed: SMOKE_WALLCLOCK_MIN,
    }
}

// ---------------------------------------------------------------------
// 14 acceptance criteria — pure-logic ones run as tests; live-infra
// ones are documented stubs that the caller wires up at run-time.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acceptance {
    Pass(&'static str),
    Fail { criterion: &'static str, reason: String },
}

impl Acceptance {
    pub fn is_pass(&self) -> bool {
        matches!(self, Acceptance::Pass(_))
    }
}

/// **#1** — every entry round-trips through validate_for_deploy with a
/// monotonic EXP_ID and is rejected from champion-locked slots.
pub fn check_1_deploy_validation(plan: &SmokePlan, current_max_exp_id: u32) -> Acceptance {
    let mut max = current_max_exp_id;
    for entry in &plan.entries {
        match entry.canon.validate_for_deploy(max) {
            Ok(()) => {}
            Err(e) => {
                return Acceptance::Fail {
                    criterion: "#1 deploy validation",
                    reason: format!("{}: {}", entry.canon, e),
                };
            }
        }
        if let Some(exp) = entry.canon.exp_id {
            max = exp;
        }
    }
    Acceptance::Pass("#1 deploy validation")
}

/// **#3** — L-R8 stdout discipline: the canon parser-side checker is
/// already covered in `canon::parse_bpb_line`. Here we just confirm
/// every entry's bpb_emit_every divides max_steps so the orchestrator
/// can budget telemetry quanta.
pub fn check_3_stdout_discipline(plan: &SmokePlan) -> Acceptance {
    for entry in &plan.entries {
        if entry.bpb_emit_every == 0 || entry.max_steps % entry.bpb_emit_every != 0 {
            return Acceptance::Fail {
                criterion: "#3 stdout discipline",
                reason: format!(
                    "{}: bpb_emit_every={} does not divide max_steps={}",
                    entry.canon, entry.bpb_emit_every, entry.max_steps
                ),
            };
        }
    }
    Acceptance::Pass("#3 stdout discipline")
}

/// **#5** — ledger uniqueness: no two entries share the same canon.
pub fn check_5_ledger_uniqueness(plan: &SmokePlan) -> Acceptance {
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for entry in &plan.entries {
        let key = entry.canon.to_string();
        if seen.insert(key.clone(), ()).is_some() {
            return Acceptance::Fail {
                criterion: "#5 ledger uniqueness",
                reason: format!("duplicate canon: {key}"),
            };
        }
    }
    Acceptance::Pass("#5 ledger uniqueness")
}

/// **#6** — L-METRIC enforcement: JEPA-T / NCA entries must commit to
/// BPB primary loss. The smoke plan models this by carrying the loss
/// kind explicitly per entry (here we test the structural side: the
/// orchestrator MUST refuse to plan a JEPA-T/NCA entry without BPB).
pub fn check_6_l_metric(plan: &SmokePlan, loss_kinds: &BTreeMap<String, String>) -> Acceptance {
    for entry in &plan.entries {
        let needs_bpb = matches!(entry.canon.model, ModelType::JepaT | ModelType::Nca);
        if !needs_bpb {
            continue;
        }
        let key = entry.canon.to_string();
        let kind = loss_kinds.get(&key).cloned().unwrap_or_default();
        if let Err(e) = entry.canon.enforce_l_metric(&kind) {
            return Acceptance::Fail {
                criterion: "#6 L-METRIC",
                reason: format!("{}: {}", entry.canon, e),
            };
        }
    }
    Acceptance::Pass("#6 L-METRIC")
}

/// **#7** — L-R9 GF16 safe domain: any GF16 entry must have h>=256.
/// Smoke plan ships FP32 only; we still test the rule lives by passing
/// in the per-entry capacity table.
pub fn check_7_l_r9(plan: &SmokePlan, capacities: &BTreeMap<String, u32>) -> Acceptance {
    for entry in &plan.entries {
        let key = entry.canon.to_string();
        let h = capacities.get(&key).copied().unwrap_or(1024);
        if let Err(e) = entry.canon.validate_with_capacity(h) {
            return Acceptance::Fail {
                criterion: "#7 L-R9",
                reason: format!("{}: {}", entry.canon, e),
            };
        }
    }
    Acceptance::Pass("#7 L-R9")
}

/// **#9** — Champion-locks neither attempted nor produced.
pub fn check_9_champion_locks_untouched(plan: &SmokePlan) -> Acceptance {
    for entry in &plan.entries {
        let exp = entry.canon.exp_id.unwrap_or(0);
        if let Some(reason) = champion_lock_reason(exp) {
            return Acceptance::Fail {
                criterion: "#9 champion locks",
                reason: format!("{}: {}", entry.canon, reason),
            };
        }
    }
    Acceptance::Pass("#9 champion locks")
}

/// **#10** — Kill-before-spin: simulated by `assert_kill_before_spin`.
pub fn check_10_kill_before_spin(plan: &SmokePlan, occupants_per_target: &BTreeMap<String, Vec<String>>) -> Acceptance {
    for entry in &plan.entries {
        let key = entry.canon.to_string();
        let occ = occupants_per_target
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let occ_refs: Vec<&str> = occ.iter().map(|s| s.as_str()).collect();
        if let Err(e) = assert_kill_before_spin(&key, &occ_refs, false) {
            return Acceptance::Fail {
                criterion: "#10 kill before spin",
                reason: format!("{e}"),
            };
        }
    }
    Acceptance::Pass("#10 kill before spin")
}

/// **#12** — EXP_ID monotonicity within the plan.
pub fn check_12_exp_id_monotonic(plan: &SmokePlan) -> Acceptance {
    let mut prev: Option<u32> = None;
    for entry in &plan.entries {
        let cur = match entry.canon.exp_id {
            Some(x) => x,
            None => {
                return Acceptance::Fail {
                    criterion: "#12 monotonic EXP_ID",
                    reason: format!("{}: missing exp_id", entry.canon),
                };
            }
        };
        if let Some(p) = prev {
            if cur <= p {
                return Acceptance::Fail {
                    criterion: "#12 monotonic EXP_ID",
                    reason: format!(
                        "{}: E{:04} not strictly greater than previous E{:04}",
                        entry.canon, cur, p
                    ),
                };
            }
        }
        prev = Some(cur);
    }
    Acceptance::Pass("#12 monotonic EXP_ID")
}

/// **#106-coupled** smoke seed range: every entry must pass
/// `assert_smoke_seed_range` and carry the `-SMOKE` marker.
pub fn check_smoke_seed_range(plan: &SmokePlan) -> Acceptance {
    for entry in &plan.entries {
        if !is_smoke(&entry.canon) {
            return Acceptance::Fail {
                criterion: "smoke marker",
                reason: format!("{}: missing -SMOKE marker", entry.canon),
            };
        }
        if let Err(e) = assert_smoke_seed_range(&entry.canon) {
            return Acceptance::Fail {
                criterion: "smoke seed range",
                reason: format!("{e}"),
            };
        }
    }
    Acceptance::Pass("smoke seed range")
}

// ---------------------------------------------------------------------
// #15 — corpus=fineweb assertion (closes #109 P0).
//
// The fleet was historically trained on tiny_shakespeare without anyone
// noticing — every BPB threshold below 1.85 is therefore meaningless
// against the real FineWeb target. Smoke MUST refuse to GO unless
// every deployed service has emitted a `corpus=<name>` line within
// `CORPUS_LOG_DEADLINE_SECONDS` of deploy AND that name equals the
// expected corpus.
//
// This check is pure: the caller (`tri smoke-race` driver) collects
// `(canon, observed_corpus, seconds_since_deploy)` triples from
// Railway logs and feeds them to `check_15_corpus_fineweb_logged`.
// We never reach out to the network here.
// ---------------------------------------------------------------------

/// Hard deadline (in seconds) by which every deployed service must
/// have logged a `corpus=...` line. Operator's brief: 2 minutes.
pub const CORPUS_LOG_DEADLINE_SECONDS: u32 = 120;

/// The exact corpus name every smoke service must report.
pub const SMOKE_REQUIRED_CORPUS: &str = "fineweb";

/// One row of operator-collected evidence for criterion #15.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusObservation {
    /// The canonical name as it appears in `SmokePlanEntry.canon`.
    pub canon: String,
    /// What the service actually emitted via `corpus=<name>` in stdout.
    /// `None` means "the deadline elapsed without any corpus line" —
    /// that is itself a hard fail (the service is silent or wrong).
    pub observed_corpus: Option<String>,
    /// Wall-clock seconds between deploy ack and the FIRST `corpus=`
    /// line being captured (or between deploy and the deadline if no
    /// line was ever observed).
    pub seconds_since_deploy: u32,
}

/// **#15** — every entry in the plan must have a matching observation
/// whose `observed_corpus == "fineweb"` AND whose
/// `seconds_since_deploy <= 120`. Closes #109 P0.
///
/// Three failure modes are reported with distinct reasons so the
/// operator can diagnose the pipe:
///   * `missing observation` — the orchestrator never saw a row for
///     this canon (the smoke-race driver dropped it).
///   * `wrong corpus` — the service logged something other than
///     `fineweb` (tiny_shakespeare, openwebtext, ...).
///   * `late corpus log` — the service logged the right name but
///     after the 2-minute deadline (slow startup, suspicious).
///   * `no corpus log` — the deadline elapsed silently.
pub fn check_15_corpus_fineweb_logged(
    plan: &SmokePlan,
    observations: &BTreeMap<String, CorpusObservation>,
) -> Acceptance {
    for entry in &plan.entries {
        let key = entry.canon.to_string();
        let obs = match observations.get(&key) {
            Some(o) => o,
            None => {
                return Acceptance::Fail {
                    criterion: "#15 corpus=fineweb logged",
                    reason: format!("{key}: missing observation (smoke-race driver did not report)"),
                };
            }
        };
        match &obs.observed_corpus {
            None => {
                return Acceptance::Fail {
                    criterion: "#15 corpus=fineweb logged",
                    reason: format!(
                        "{key}: no corpus log within {}s deadline",
                        CORPUS_LOG_DEADLINE_SECONDS
                    ),
                };
            }
            Some(name) if name != SMOKE_REQUIRED_CORPUS => {
                return Acceptance::Fail {
                    criterion: "#15 corpus=fineweb logged",
                    reason: format!(
                        "{key}: wrong corpus={name}, expected {SMOKE_REQUIRED_CORPUS} (closes #109 P0)"
                    ),
                };
            }
            Some(_) => {
                if obs.seconds_since_deploy > CORPUS_LOG_DEADLINE_SECONDS {
                    return Acceptance::Fail {
                        criterion: "#15 corpus=fineweb logged",
                        reason: format!(
                            "{key}: late corpus log {}s > {}s deadline",
                            obs.seconds_since_deploy, CORPUS_LOG_DEADLINE_SECONDS
                        ),
                    };
                }
            }
        }
    }
    Acceptance::Pass("#15 corpus=fineweb logged")
}

/// Build the L-METRIC loss-kinds map that the smoke config commits to:
/// every JEPA-T / NCA entry pins `"bpb"` as the primary loss.
pub fn smoke_default_loss_kinds(plan: &SmokePlan) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in &plan.entries {
        if matches!(entry.canon.model, ModelType::JepaT | ModelType::Nca) {
            out.insert(entry.canon.to_string(), "bpb".to_string());
        }
    }
    out
}

/// Plan-level run: collect all pure-logic acceptance results.
///
/// Live-infra criteria (#2 health-check, #4 ingest count, #8 leaderboard
/// rank, #11 cross-account kill, #13 idempotent re-deploy, #14 graceful
/// kill on smoke end) are documented in the README as runtime checks
/// the operator's `tri smoke-race` driver verifies; they are not
/// expressible without a network and are out of scope for this module.
pub fn run_pure_acceptance(plan: &SmokePlan, current_max_exp_id: u32) -> Vec<Acceptance> {
    vec![
        check_1_deploy_validation(plan, current_max_exp_id),
        check_3_stdout_discipline(plan),
        check_5_ledger_uniqueness(plan),
        check_6_l_metric(plan, &smoke_default_loss_kinds(plan)),
        check_7_l_r9(plan, &BTreeMap::new()),
        check_9_champion_locks_untouched(plan),
        check_10_kill_before_spin(plan, &BTreeMap::new()),
        check_12_exp_id_monotonic(plan),
        check_smoke_seed_range(plan),
    ]
}

/// Plan-level run including the live-infra-derived #15 corpus check.
/// Caller passes the observation map after the smoke-race driver has
/// finished the 2-minute warm-up window.
pub fn run_pure_acceptance_with_corpus(
    plan: &SmokePlan,
    current_max_exp_id: u32,
    corpus_observations: &BTreeMap<String, CorpusObservation>,
) -> Vec<Acceptance> {
    let mut out = run_pure_acceptance(plan, current_max_exp_id);
    out.push(check_15_corpus_fineweb_logged(plan, corpus_observations));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_plan() -> SmokePlan {
        // First production E-id is 4 (champion locks claim 1..=4);
        // smoke claims 500..=520 (21 entries strictly above 4).
        build_default_plan(SMOKE_FIRST_EXP_ID)
    }

    #[test]
    fn plan_has_exactly_21_entries() {
        let plan = fixture_plan();
        assert_eq!(plan.entries.len(), 21);
    }

    #[test]
    fn plan_covers_seven_experiment_tags() {
        let plan = fixture_plan();
        let mut tags: std::collections::BTreeSet<&str> =
            std::collections::BTreeSet::new();
        for e in &plan.entries {
            tags.insert(e.experiment_tag);
        }
        assert_eq!(tags.len(), 7);
        assert!(tags.contains("WSD"));
        assert!(tags.contains("BS8"));
        assert!(tags.contains("GRADFIX_JEPAT"));
        assert!(tags.contains("GRADFIX_NCA"));
        assert!(tags.contains("EMA10"));
        assert!(tags.contains("h512"));
        assert!(tags.contains("h768"));
    }

    #[test]
    fn plan_uses_only_smoke_seed_range() {
        let plan = fixture_plan();
        for e in &plan.entries {
            let rng = e.canon.rng.unwrap();
            assert!(SMOKE_SEED_RANGE.contains(&rng), "rng={rng} out of range");
        }
    }

    #[test]
    fn acceptance_1_deploy_validation_passes() {
        let plan = fixture_plan();
        let r = check_1_deploy_validation(&plan, 4);
        assert!(r.is_pass(), "got {r:?}");
    }

    #[test]
    fn acceptance_3_stdout_discipline_passes() {
        let plan = fixture_plan();
        assert!(check_3_stdout_discipline(&plan).is_pass());
    }

    #[test]
    fn acceptance_5_ledger_uniqueness_passes() {
        let plan = fixture_plan();
        assert!(check_5_ledger_uniqueness(&plan).is_pass());
    }

    #[test]
    fn acceptance_5_catches_planted_duplicate() {
        let mut plan = fixture_plan();
        let dup = plan.entries[0].clone();
        plan.entries.push(dup);
        let r = check_5_ledger_uniqueness(&plan);
        assert!(!r.is_pass());
    }

    #[test]
    fn acceptance_6_l_metric_passes_with_bpb_kinds() {
        let plan = fixture_plan();
        let mut kinds: BTreeMap<String, String> = BTreeMap::new();
        for e in &plan.entries {
            if matches!(e.canon.model, ModelType::JepaT | ModelType::Nca) {
                kinds.insert(e.canon.to_string(), "bpb".to_string());
            }
        }
        assert!(check_6_l_metric(&plan, &kinds).is_pass());
    }

    #[test]
    fn acceptance_6_l_metric_catches_mse_proxy_on_jepa_t() {
        let plan = fixture_plan();
        let mut kinds: BTreeMap<String, String> = BTreeMap::new();
        for e in &plan.entries {
            if matches!(e.canon.model, ModelType::JepaT) {
                kinds.insert(e.canon.to_string(), "mse".to_string());
            }
        }
        let r = check_6_l_metric(&plan, &kinds);
        assert!(!r.is_pass(), "expected fail, got {r:?}");
    }

    #[test]
    fn acceptance_9_champion_locks_untouched() {
        let plan = fixture_plan();
        assert!(check_9_champion_locks_untouched(&plan).is_pass());
    }

    #[test]
    fn acceptance_9_catches_attempt_to_redeploy_champion_slot() {
        let mut plan = fixture_plan();
        // mutate one entry to point at the locked E0001
        plan.entries[0].canon.exp_id = Some(1);
        let r = check_9_champion_locks_untouched(&plan);
        assert!(!r.is_pass());
    }

    #[test]
    fn acceptance_10_kill_before_spin_passes_with_empty_occupants() {
        let plan = fixture_plan();
        assert!(check_10_kill_before_spin(&plan, &BTreeMap::new()).is_pass());
    }

    #[test]
    fn acceptance_10_catches_occupied_slot() {
        let plan = fixture_plan();
        let mut occ = BTreeMap::new();
        occ.insert(
            plan.entries[0].canon.to_string(),
            vec!["trios-train-old-svc".to_string()],
        );
        assert!(!check_10_kill_before_spin(&plan, &occ).is_pass());
    }

    #[test]
    fn acceptance_12_exp_id_strictly_monotonic() {
        let plan = fixture_plan();
        assert!(check_12_exp_id_monotonic(&plan).is_pass());
    }

    #[test]
    fn acceptance_12_catches_repeated_exp_id() {
        let mut plan = fixture_plan();
        plan.entries[1].canon.exp_id = plan.entries[0].canon.exp_id;
        assert!(!check_12_exp_id_monotonic(&plan).is_pass());
    }

    #[test]
    fn acceptance_smoke_seed_range_passes_for_full_plan() {
        let plan = fixture_plan();
        assert!(check_smoke_seed_range(&plan).is_pass());
    }

    #[test]
    fn run_pure_acceptance_all_pass_on_canonical_plan() {
        let plan = fixture_plan();
        let results = run_pure_acceptance(&plan, 4);
        for r in &results {
            assert!(r.is_pass(), "criterion failed: {r:?}");
        }
        assert_eq!(results.len(), 9);
    }

    #[test]
    fn first_smoke_exp_id_strictly_above_champion_locks() {
        // Champions hold E0001..E0004; smoke must start strictly above.
        assert!(SMOKE_FIRST_EXP_ID > 4);
    }

    // ----- #15 corpus=fineweb tests (closes #109 P0) -----

    fn corpus_obs_all(plan: &SmokePlan, name: &str, secs: u32) -> BTreeMap<String, CorpusObservation> {
        let mut m = BTreeMap::new();
        for entry in &plan.entries {
            let k = entry.canon.to_string();
            m.insert(
                k.clone(),
                CorpusObservation {
                    canon: k,
                    observed_corpus: Some(name.to_string()),
                    seconds_since_deploy: secs,
                },
            );
        }
        m
    }

    #[test]
    fn acceptance_15_passes_when_every_service_logs_fineweb_within_deadline() {
        let plan = fixture_plan();
        let obs = corpus_obs_all(&plan, "fineweb", 30);
        assert!(check_15_corpus_fineweb_logged(&plan, &obs).is_pass());
    }

    #[test]
    fn acceptance_15_fails_when_any_service_logs_tiny_shakespeare() {
        let plan = fixture_plan();
        let mut obs = corpus_obs_all(&plan, "fineweb", 30);
        // Poison one observation with the historical bug.
        let bad_key = plan.entries[3].canon.to_string();
        obs.insert(
            bad_key.clone(),
            CorpusObservation {
                canon: bad_key,
                observed_corpus: Some("tiny_shakespeare".to_string()),
                seconds_since_deploy: 30,
            },
        );
        let r = check_15_corpus_fineweb_logged(&plan, &obs);
        assert!(!r.is_pass(), "expected fail on tiny_shakespeare, got {r:?}");
        if let Acceptance::Fail { reason, .. } = r {
            assert!(reason.contains("wrong corpus=tiny_shakespeare"), "reason={reason}");
            assert!(reason.contains("#109"), "reason={reason}");
        }
    }

    #[test]
    fn acceptance_15_fails_when_observation_missing() {
        let plan = fixture_plan();
        let mut obs = corpus_obs_all(&plan, "fineweb", 30);
        let drop_key = plan.entries[5].canon.to_string();
        obs.remove(&drop_key);
        let r = check_15_corpus_fineweb_logged(&plan, &obs);
        assert!(!r.is_pass());
        if let Acceptance::Fail { reason, .. } = r {
            assert!(reason.contains("missing observation"), "reason={reason}");
        }
    }

    #[test]
    fn acceptance_15_fails_on_silent_service_at_deadline() {
        let plan = fixture_plan();
        let mut obs = corpus_obs_all(&plan, "fineweb", 30);
        let silent_key = plan.entries[7].canon.to_string();
        obs.insert(
            silent_key.clone(),
            CorpusObservation {
                canon: silent_key,
                observed_corpus: None,
                seconds_since_deploy: CORPUS_LOG_DEADLINE_SECONDS,
            },
        );
        let r = check_15_corpus_fineweb_logged(&plan, &obs);
        assert!(!r.is_pass());
        if let Acceptance::Fail { reason, .. } = r {
            assert!(reason.contains("no corpus log"), "reason={reason}");
        }
    }

    #[test]
    fn acceptance_15_fails_when_corpus_logged_after_deadline() {
        let plan = fixture_plan();
        let mut obs = corpus_obs_all(&plan, "fineweb", 30);
        let slow_key = plan.entries[10].canon.to_string();
        obs.insert(
            slow_key.clone(),
            CorpusObservation {
                canon: slow_key,
                observed_corpus: Some("fineweb".to_string()),
                seconds_since_deploy: CORPUS_LOG_DEADLINE_SECONDS + 1,
            },
        );
        let r = check_15_corpus_fineweb_logged(&plan, &obs);
        assert!(!r.is_pass());
        if let Acceptance::Fail { reason, .. } = r {
            assert!(reason.contains("late corpus log"), "reason={reason}");
        }
    }

    #[test]
    fn run_pure_acceptance_with_corpus_yields_ten_results() {
        let plan = fixture_plan();
        let obs = corpus_obs_all(&plan, "fineweb", 30);
        let results = run_pure_acceptance_with_corpus(&plan, 4, &obs);
        assert_eq!(results.len(), 10, "9 pure + 1 corpus = 10");
        for r in &results {
            assert!(r.is_pass(), "criterion failed: {r:?}");
        }
    }

    #[test]
    fn corpus_constants_match_operator_brief() {
        assert_eq!(CORPUS_LOG_DEADLINE_SECONDS, 120);
        assert_eq!(SMOKE_REQUIRED_CORPUS, "fineweb");
    }
}

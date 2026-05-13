//! Issue #63 — `render_leaderboard()` and the R0 invariant.
//!
//! The gardener's R0 invariant: **every tick prints a leaderboard**,
//! even when every BPB source is empty. Failure modes (no telemetry,
//! probe-failed accounts, network errors) become rows in the
//! "tracking" section, not panics or silent skips.

#[cfg(test)]
use chrono::TimeZone;
use chrono::{DateTime, Duration, Utc};

use crate::bpb_source::BpbSample;
#[cfg(test)]
use crate::bpb_source::SourceTag;
use crate::ledger::ARCHITECTURAL_FLOOR_BPB;

pub const RACE_START_RFC3339: &str = "2026-04-27T18:00:00Z";
pub const GATE2_DEADLINE_RFC3339: &str = "2026-04-30T23:59:00Z";
pub const GATE2_TARGET_BPB: f64 = 1.85;
pub const RUNG1_OFFSET_HOURS: i64 = 12;
pub const RUNG2_OFFSET_HOURS: i64 = 18;
pub const RUNG3_OFFSET_HOURS: i64 = 28;

/// Per-tick fleet picture passed alongside the BPB samples.
#[derive(Debug, Clone)]
pub struct FleetStatus {
    pub acc0: AccountProbe,
    pub acc1: AccountProbe,
    pub acc2: AccountProbe,
}

#[derive(Debug, Clone)]
pub struct AccountProbe {
    pub label: &'static str,
    pub state: ProbeState,
    pub services_observed: u32,
    pub services_expected: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    Ok,
    NotAuthorized,
    NetworkError,
    NotProbed,
}

impl AccountProbe {
    fn render_cell(&self) -> String {
        let state = match self.state {
            ProbeState::Ok => "OK",
            ProbeState::NotAuthorized => "FAIL (Not Authorized)",
            ProbeState::NetworkError => "FAIL (network)",
            ProbeState::NotProbed => "—",
        };
        format!(
            "{}: {} · services {}/{}",
            self.label, state, self.services_observed, self.services_expected
        )
    }
}

/// Seeds the gardener expects to track even when no BPB has been
/// observed. Drives the "tracking" rows so the operator sees the
/// fleet-shape at every tick.
#[derive(Debug, Clone)]
pub struct ExpectedSeed {
    pub seed: u32,
    pub lane: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct LeaderboardCtx {
    pub now: DateTime<Utc>,
    pub samples: Vec<BpbSample>,
    pub expected: Vec<ExpectedSeed>,
    pub fleet: FleetStatus,
}

/// R0 entry point: produce the markdown leaderboard for one tick.
///
/// **Never panics, never errors out.** The function takes already-fetched
/// samples plus a fleet snapshot; merging and probing happen earlier.
pub fn render_leaderboard(ctx: &LeaderboardCtx) -> String {
    let mut s = String::new();
    let race_start: DateTime<Utc> = RACE_START_RFC3339.parse().expect("race-start parses");
    let elapsed = ctx.now.signed_duration_since(race_start);
    let t_hours = elapsed.num_minutes() as f64 / 60.0;

    let local = ctx
        .now
        .with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap());

    s.push_str(&format!(
        "## LIVE LEADERBOARD — T+{:.2}h ({} +07)\n\n",
        t_hours,
        local.format("%H:%M")
    ));

    // --- ranked table ---
    s.push_str("| Rank | Seed | Lane | Steps | BPB | Δ→Gate-2 | Trend | Status |\n");
    s.push_str("|---:|---:|---|---:|---:|---:|:---:|---|\n");

    let mut ranked: Vec<&BpbSample> = ctx.samples.iter().collect();
    ranked.sort_by(|a, b| {
        a.bpb
            .partial_cmp(&b.bpb)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let champion_bpb = ranked.first().map(|s| s.bpb);
    for (i, sample) in ranked.iter().enumerate() {
        let rank = i + 1;
        let medal = match rank {
            1 => "🥇 1".to_string(),
            2 => "🥈 2".to_string(),
            3 => "🥉 3".to_string(),
            _ => format!("{}", rank),
        };
        let bpb_cell = if Some(sample.bpb) == champion_bpb {
            format!("**{:.4}**", sample.bpb)
        } else {
            format!("{:.4}", sample.bpb)
        };
        let delta = sample.bpb - GATE2_TARGET_BPB;
        let trend = trend_arrow(sample.bpb);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | +{:.4} | {} | observed ({}) |\n",
            medal,
            sample.seed,
            sample.lane,
            sample.step,
            bpb_cell,
            delta,
            trend,
            sample.source.as_str(),
        ));
    }

    // --- tracking rows for expected seeds without samples ---
    let observed: std::collections::HashSet<(u32, String)> =
        ranked.iter().map(|s| (s.seed, s.lane.clone())).collect();
    for exp in &ctx.expected {
        if observed.contains(&(exp.seed, exp.lane.clone())) {
            continue;
        }
        s.push_str(&format!(
            "| — | {} | {} | — | tracking | — | ⏳ | warmup · {} |\n",
            exp.seed, exp.lane, exp.note,
        ));
    }

    // --- footer: fleet + champion + ETAs ---
    s.push_str("\n### Fleet status\n\n");
    s.push_str(&format!("- {}\n", ctx.fleet.acc0.render_cell()));
    s.push_str(&format!("- {}\n", ctx.fleet.acc1.render_cell()));
    s.push_str(&format!("- {}\n", ctx.fleet.acc2.render_cell()));

    s.push_str("\n### Champion + Gate-2\n\n");
    if let Some(c) = champion_bpb {
        let gap = c - GATE2_TARGET_BPB;
        let quorum_at_gate2 = ranked.iter().filter(|x| x.bpb < GATE2_TARGET_BPB).count();
        s.push_str(&format!(
            "- Champion BPB: **{:.4}** · gap to Gate-2: +{:.4}\n",
            c, gap
        ));
        s.push_str(&format!(
            "- Quorum at Gate-2 target: {}/3 (need 3 seeds < {})\n",
            quorum_at_gate2, GATE2_TARGET_BPB
        ));
        s.push_str(&format!(
            "- Architectural floor (cull-safety): {:.2}\n",
            ARCHITECTURAL_FLOOR_BPB
        ));
    } else {
        s.push_str("- **NO BPB OBSERVED THIS TICK** — all sources empty.\n");
        s.push_str("  Reasons (R5 honest): `bpb_samples` table missing (#62), Railway logs require account-scoped tokens (#61 P0), and recent ALPHA comments may not include `BPB=… seed=… step=…` lines.\n");
    }

    s.push_str("\n### ETA\n\n");
    s.push_str("| Marker | UTC | T+ |\n|---|---|---|\n");
    s.push_str(&format!(
        "| Now | {} | {} |\n",
        ctx.now.format("%Y-%m-%d %H:%MZ"),
        format_t_plus(elapsed)
    ));
    push_eta(
        &mut s,
        race_start,
        RUNG1_OFFSET_HOURS,
        "Rung-1 (cull > 2.30)",
        ctx.now,
    );
    push_eta(
        &mut s,
        race_start,
        RUNG2_OFFSET_HOURS,
        "Rung-2 (cull > 2.20)",
        ctx.now,
    );
    push_eta(
        &mut s,
        race_start,
        RUNG3_OFFSET_HOURS,
        "Rung-3 (cull > 2.05)",
        ctx.now,
    );
    let deadline: DateTime<Utc> = GATE2_DEADLINE_RFC3339.parse().unwrap();
    let dl_elapsed = deadline.signed_duration_since(race_start);
    s.push_str(&format!(
        "| Gate-2 deadline | {} | {} |\n",
        deadline.format("%Y-%m-%d %H:%MZ"),
        format_t_plus(dl_elapsed)
    ));

    s.push_str("\n`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`\n");
    s
}

fn trend_arrow(bpb: f64) -> &'static str {
    if bpb < GATE2_TARGET_BPB {
        "↓"
    } else if bpb >= 2.6 {
        "↑"
    } else {
        "→"
    }
}

fn push_eta(
    out: &mut String,
    race_start: DateTime<Utc>,
    offset_h: i64,
    label: &str,
    now: DateTime<Utc>,
) {
    let target = race_start + Duration::hours(offset_h);
    let delta = target.signed_duration_since(now);
    let label_cell = if delta.num_seconds() < 0 {
        format!("{} (passed)", label)
    } else {
        label.to_string()
    };
    let from_start = target.signed_duration_since(race_start);
    out.push_str(&format!(
        "| {} | {} | {} |\n",
        label_cell,
        target.format("%Y-%m-%d %H:%MZ"),
        format_t_plus(from_start),
    ));
}

fn format_t_plus(d: Duration) -> String {
    let h = d.num_minutes() as f64 / 60.0;
    if h < 0.0 {
        format!("T{:.2}h", h)
    } else {
        format!("T+{:.2}h", h)
    }
}

/// Helper used by `loop_once` to seed the tracking rows even when
/// nothing comes back from any source.
///
/// Post-T+11.5h pivot: the 9 attention/JEPA Phase-1 lanes are flagged
/// `cull-pending` (their architecture maxes at ≈2.19, see the new
/// champion train_v2). They stay on the leaderboard as warmup-tracking
/// rows for now, but the operator should kill the underlying Railway
/// services and re-deploy seeds 42/43/44 on train_v2 (see
/// `docs/POSTMORTEM_GATE2_LOCAL_WIN.md`).
pub fn default_phase1_expected() -> Vec<ExpectedSeed> {
    let mk = |seed: u32, lane: &str, note: &str| ExpectedSeed {
        seed,
        lane: lane.to_string(),
        note: note.to_string(),
    };
    vec![
        // train_v2 quorum slots (target Gate-2 OFFICIAL): pending portage
        mk(
            42,
            "train_v2 (h=1024 ctx=12 14-gram WT+resid)",
            "local Mac champion @ 94.5K BPB=1.8921 — Railway portage pending",
        ),
        mk(
            43,
            "train_v2 (h=1024 ctx=12 14-gram WT+resid)",
            "Railway portage pending (quorum-3)",
        ),
        mk(
            44,
            "train_v2 (h=1024 ctx=12 14-gram WT+resid)",
            "Railway portage pending (quorum-3)",
        ),
        // Old Phase-1 attention/JEPA fleet — architecture lost the race; cull-pending
        mk(
            210,
            "L1 attn-backward (cull-pending: arch lost)",
            "Acc1 svc a2a24d1c",
        ),
        mk(
            211,
            "L1 attn-backward (cull-pending: arch lost)",
            "Acc1 svc fcd0cfbe",
        ),
        mk(
            212,
            "L1 attn-backward (cull-pending: arch lost)",
            "Acc1 svc 861b9501",
        ),
        mk(
            220,
            "L2 JEPA-T (cull-pending: arch lost)",
            "Acc1 svc eb9d7525",
        ),
        mk(
            221,
            "L2 JEPA-T (cull-pending: arch lost)",
            "Acc1 svc 05dd3cb0",
        ),
        mk(
            222,
            "L2 JEPA-T (cull-pending: arch lost)",
            "Acc1 svc e32af244",
        ),
        mk(
            240,
            "L4 h=2000 (cull-pending: arch lost)",
            "Acc1 svc c9c5324d",
        ),
        mk(
            241,
            "L4 h=2000 (cull-pending: arch lost)",
            "Acc1 svc 8e64cf14",
        ),
        mk(
            242,
            "L4 h=2000 (cull-pending: arch lost)",
            "Acc1 svc 3de0f6ad",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 5, 16, 0).unwrap()
    }

    fn empty_fleet() -> FleetStatus {
        FleetStatus {
            acc0: AccountProbe {
                label: "Acc0",
                state: ProbeState::NotAuthorized,
                services_observed: 0,
                services_expected: 6,
            },
            acc1: AccountProbe {
                label: "Acc1",
                state: ProbeState::Ok,
                services_observed: 9,
                services_expected: 9,
            },
            acc2: AccountProbe {
                label: "Acc2",
                state: ProbeState::NotAuthorized,
                services_observed: 0,
                services_expected: 6,
            },
        }
    }

    #[test]
    fn renders_with_zero_samples_and_explains_why() {
        let ctx = LeaderboardCtx {
            now: fixed_now(),
            samples: Vec::new(),
            expected: default_phase1_expected(),
            fleet: empty_fleet(),
        };
        let out = render_leaderboard(&ctx);
        assert!(out.contains("LIVE LEADERBOARD"));
        assert!(out.contains("NO BPB OBSERVED"));
        assert!(out.contains("Acc1: OK"));
        assert!(out.contains("Acc0: FAIL"));
        // tracking rows for all 12 expected seeds (3 train_v2 quorum +
        // 9 cull-pending Phase-1 attention/JEPA leftovers).
        for seed in [42, 43, 44, 210, 211, 212, 220, 221, 222, 240, 241, 242] {
            assert!(
                out.contains(&format!("| {seed} |")),
                "missing tracking row for {seed}"
            );
        }
        // Champion train_v2 architectural pivot must be visible in the
        // tracking rows so the operator never forgets it.
        assert!(out.contains("train_v2"));
        assert!(out.contains("BPB=1.8921"));
    }

    #[test]
    fn renders_champion_and_quorum_with_known_samples() {
        let now = fixed_now();
        let mk = |seed, bpb, step| BpbSample {
            seed,
            lane: "champion".to_string(),
            step,
            bpb,
            ts: now,
            source: SourceTag::Manual,
        };
        let ctx = LeaderboardCtx {
            now,
            samples: vec![
                mk(43, 2.1919, 81000),
                mk(44, 2.2024, 81000),
                mk(45, 2.1944, 81000),
            ],
            expected: vec![],
            fleet: empty_fleet(),
        };
        let out = render_leaderboard(&ctx);
        assert!(out.contains("**2.1919**"));
        assert!(out.contains("Champion BPB: **2.1919**"));
        assert!(out.contains("Quorum at Gate-2 target: 0/3"));
        assert!(out.contains("🥇 1"));
    }

    #[test]
    fn r0_invariant_does_not_panic_on_pathological_input() {
        // Empty fleet, empty samples, empty expected → still renders.
        let ctx = LeaderboardCtx {
            now: fixed_now(),
            samples: vec![],
            expected: vec![],
            fleet: FleetStatus {
                acc0: AccountProbe {
                    label: "Acc0",
                    state: ProbeState::NotProbed,
                    services_observed: 0,
                    services_expected: 0,
                },
                acc1: AccountProbe {
                    label: "Acc1",
                    state: ProbeState::NotProbed,
                    services_observed: 0,
                    services_expected: 0,
                },
                acc2: AccountProbe {
                    label: "Acc2",
                    state: ProbeState::NotProbed,
                    services_observed: 0,
                    services_expected: 0,
                },
            },
        };
        let out = render_leaderboard(&ctx);
        assert!(out.contains("LIVE LEADERBOARD"));
    }

    #[test]
    fn eta_section_includes_rung1_rung2_rung3_and_deadline() {
        let ctx = LeaderboardCtx {
            now: fixed_now(),
            samples: vec![],
            expected: vec![],
            fleet: empty_fleet(),
        };
        let out = render_leaderboard(&ctx);
        assert!(out.contains("Rung-1"));
        assert!(out.contains("Rung-2"));
        assert!(out.contains("Rung-3"));
        assert!(out.contains("Gate-2 deadline"));
    }
}

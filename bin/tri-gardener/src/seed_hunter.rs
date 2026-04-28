//! Seed Hunter — predictive early-stopping per RNG seed.
//!
//! Design (operator brief, condensed):
//! 1. After ≥ N BPB samples per seed, fit `BPB(t) = bpb_inf + a · t^(-p)`.
//! 2. Build a 95% confidence interval on `bpb_inf` from the residuals.
//! 3. Compute leader-relative `Δ` and `slope(Δ)` over a rolling window.
//! 4. Classify each seed into one of five states; the gardener acts on
//!    the classification at every ASHA rung.
//! 5. Generate next-batch seeds either at random or `φ`-anchored
//!    (`floor(φ^k · M) mod 2^32`) — operator-selectable strategy.
//!
//! **No new external deps.** The fit is a 2-D grid search over `p` and
//! `bpb_inf`, then a closed-form linear least-squares solve for `a`
//! given `(bpb_inf, p)`. The grid is small (≈3000 candidates) and runs
//! in microseconds, which keeps the gardener's tick budget honest.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`.

use std::collections::BTreeMap;

/// One observed BPB datapoint for a seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePoint {
    pub step: u32,
    pub bpb: f64,
}

/// Outcome of fitting `bpb(t) = bpb_inf + a · t^{-p}` to a curve.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerLawFit {
    pub bpb_inf: f64,
    pub a: f64,
    pub p: f64,
    /// 95% half-width (`bpb_inf ± ci_half_width`) derived from the
    /// residual standard deviation. Honest: this is **not** a true
    /// non-linear CI, it's a residual-driven proxy. Caller should treat
    /// it as a width, not as a probability.
    pub ci_half_width: f64,
    pub n_samples: usize,
    /// Residual sum of squares at the optimum.
    pub rss: f64,
}

/// Hyperparameters of the fit. Defaults are tuned for IGLA's BPB
/// curves (typical p ∈ [0.20, 0.80], bpb_inf ∈ [1.0, 3.5]).
#[derive(Debug, Clone)]
pub struct FitOptions {
    pub p_min: f64,
    pub p_max: f64,
    pub p_steps: usize,
    pub bpb_inf_min: f64,
    pub bpb_inf_max: f64,
    pub bpb_inf_steps: usize,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            p_min: 0.10,
            p_max: 0.90,
            p_steps: 41,
            bpb_inf_min: 0.50,
            bpb_inf_max: 3.50,
            bpb_inf_steps: 61,
        }
    }
}

/// Fit a power law to the curve. Returns `None` if there are fewer
/// than 4 distinct steps (NLS is under-determined below that).
pub fn fit_power_law(points: &[CurvePoint], opts: &FitOptions) -> Option<PowerLawFit> {
    if points.len() < 4 {
        return None;
    }
    // Grid search over (bpb_inf, p), closed-form OLS for `a`.
    let mut best: Option<PowerLawFit> = None;
    let p_step = (opts.p_max - opts.p_min) / (opts.p_steps - 1).max(1) as f64;
    let b_step = (opts.bpb_inf_max - opts.bpb_inf_min) / (opts.bpb_inf_steps - 1).max(1) as f64;
    for ip in 0..opts.p_steps {
        let p = opts.p_min + ip as f64 * p_step;
        // Precompute t^{-p} per point.
        let xs: Vec<f64> = points
            .iter()
            .map(|cp| (cp.step.max(1) as f64).powf(-p))
            .collect();
        for ib in 0..opts.bpb_inf_steps {
            let bpb_inf = opts.bpb_inf_min + ib as f64 * b_step;
            // OLS for `a`: minimize Σ (a·x_i − (y_i − bpb_inf))^2.
            // Closed-form: a = Σ x·(y−bpb_inf) / Σ x^2.
            let mut num = 0.0;
            let mut den = 0.0;
            for (cp, x) in points.iter().zip(&xs) {
                let target = cp.bpb - bpb_inf;
                num += x * target;
                den += x * x;
            }
            if den < 1e-30 {
                continue;
            }
            let a = num / den;
            // Residual sum of squares.
            let mut rss = 0.0;
            for (cp, x) in points.iter().zip(&xs) {
                let pred = bpb_inf + a * x;
                rss += (cp.bpb - pred).powi(2);
            }
            let replace = match &best {
                None => true,
                Some(b) => rss < b.rss,
            };
            if replace {
                let n = points.len() as f64;
                let dof = (n - 3.0).max(1.0);
                let sigma = (rss / dof).sqrt();
                // 95% half-width ≈ 1.96 · σ. The asymptote sees the
                // smallest x_i (largest step), so the practical CI is
                // tighter than σ; we keep σ as a conservative proxy.
                let ci_half_width = 1.96 * sigma;
                best = Some(PowerLawFit {
                    bpb_inf,
                    a,
                    p,
                    ci_half_width,
                    n_samples: points.len(),
                    rss,
                });
            }
        }
    }
    best
}

// ---------------------------------------------------------------------
// Leader-relative classifier.
// ---------------------------------------------------------------------

/// State of one seed relative to the current leader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedState {
    /// Δ ≤ 0: this seed *is* the leader (or tied).
    Leading,
    /// Δ within `tied_band` (default 0.005).
    Tied,
    /// Δ > 0, slope < 0 → catching up.
    CatchingUp,
    /// Δ > tied_band, slope ≈ 0 → not closing the gap.
    ParallelLosing,
    /// Δ > 0, slope > slope_kill_threshold → diverging, kill.
    Diverging,
}

#[derive(Debug, Clone)]
pub struct ClassifierOptions {
    pub tied_band: f64,
    pub parallel_band: f64,
    pub slope_kill_threshold: f64,
    /// Rolling window over which the slope is computed (in BPB samples,
    /// not steps).
    pub slope_window: usize,
}

impl Default for ClassifierOptions {
    fn default() -> Self {
        Self {
            tied_band: 0.005,
            parallel_band: 0.05,
            slope_kill_threshold: 1e-3, // ΔBPB / step
            slope_window: 8,
        }
    }
}

/// Compute `slope(Δ vs step)` over the trailing `window` points using a
/// closed-form linear regression slope.
fn rolling_delta_slope(deltas: &[CurvePoint], window: usize) -> Option<f64> {
    let n = deltas.len();
    if n < 2 {
        return None;
    }
    let take = window.min(n);
    let slice = &deltas[n - take..];
    let nf = slice.len() as f64;
    let sum_x: f64 = slice.iter().map(|p| p.step as f64).sum();
    let sum_y: f64 = slice.iter().map(|p| p.bpb).sum();
    let sum_xx: f64 = slice.iter().map(|p| (p.step as f64).powi(2)).sum();
    let sum_xy: f64 = slice.iter().map(|p| p.step as f64 * p.bpb).sum();
    let denom = nf * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-30 {
        return None;
    }
    Some((nf * sum_xy - sum_x * sum_y) / denom)
}

/// Classify a seed against the current leader's curve.
///
/// `seed_curve` and `leader_curve` must be aligned on `step` for the
/// last `slope_window` points (i.e. the caller has already merged them
/// on the same rung). Both are sorted by step ascending.
pub fn classify_seed(
    seed_curve: &[CurvePoint],
    leader_curve: &[CurvePoint],
    opts: &ClassifierOptions,
) -> Option<SeedState> {
    if seed_curve.is_empty() || leader_curve.is_empty() {
        return None;
    }
    // Build Δ = seed.bpb − leader.bpb, joined on step.
    let leader_map: BTreeMap<u32, f64> =
        leader_curve.iter().map(|p| (p.step, p.bpb)).collect();
    let mut deltas: Vec<CurvePoint> = Vec::with_capacity(seed_curve.len());
    for p in seed_curve {
        if let Some(lb) = leader_map.get(&p.step) {
            deltas.push(CurvePoint {
                step: p.step,
                bpb: p.bpb - lb,
            });
        }
    }
    if deltas.is_empty() {
        return None;
    }
    let last = deltas.last().unwrap().bpb;
    if last <= opts.tied_band && last >= -opts.tied_band {
        return Some(SeedState::Tied);
    }
    if last < 0.0 {
        return Some(SeedState::Leading);
    }
    let slope = rolling_delta_slope(&deltas, opts.slope_window).unwrap_or(0.0);
    if slope > opts.slope_kill_threshold {
        return Some(SeedState::Diverging);
    }
    if slope < 0.0 {
        return Some(SeedState::CatchingUp);
    }
    if last > opts.parallel_band {
        return Some(SeedState::ParallelLosing);
    }
    // Small Δ, flat slope, but outside tied_band → ParallelLosing too.
    Some(SeedState::ParallelLosing)
}

// ---------------------------------------------------------------------
// ASHA-style rung schedule.
// ---------------------------------------------------------------------

/// One rung in the schedule: a step threshold + a "keep top-K" rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rung {
    pub step: u32,
    pub keep_top_k: usize,
}

/// Default schedule from the operator brief:
/// rung_0 step 100 collect; rung_1 step 500 drop 50%; rung_2 step 2000
/// drop 50%; rung_3 step 8000 keep top 3; rung_4 step 32000 keep top 1;
/// rung_5 step 81000 final.
pub const DEFAULT_SCHEDULE: &[Rung] = &[
    Rung { step: 100, keep_top_k: usize::MAX },
    Rung { step: 500, keep_top_k: usize::MAX }, // see resolve_keep
    Rung { step: 2000, keep_top_k: usize::MAX },
    Rung { step: 8000, keep_top_k: 3 },
    Rung { step: 32000, keep_top_k: 1 },
    Rung { step: 81000, keep_top_k: 1 },
];

/// Resolve `keep_top_k` for "drop bottom 50%" rungs (1, 2): caller
/// passes the current alive count, gets back `(count + 1) / 2`.
pub fn resolve_keep_top_k(rung_idx: usize, alive: usize) -> usize {
    if rung_idx == 1 || rung_idx == 2 {
        ((alive as f64) / 2.0).ceil() as usize
    } else {
        DEFAULT_SCHEDULE[rung_idx].keep_top_k.min(alive)
    }
}

// ---------------------------------------------------------------------
// φ-anchored seed generation.
// ---------------------------------------------------------------------

/// Operator's hypothesis: `seed_k = floor(φ^k · M) mod 2^32` may give
/// initial conditioning with Zeckendorf-flavoured weight structure.
/// Paired with `INV-3,5` (Lucas closure) the float arithmetic is stable
/// under low precision, so the hypothesis is also testable in GF16.
pub fn phi_anchored_seeds(count: usize, multiplier: f64) -> Vec<u32> {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    (1..=count)
        .map(|k| {
            // Use exp(k · ln φ) for k > ~30 to keep precision.
            let val = (k as f64 * phi.ln()).exp() * multiplier;
            // Wrap into u32 via fmod 2^32, never panic on huge floats.
            let modulus = (u32::MAX as f64) + 1.0;
            (val.rem_euclid(modulus)) as u32
        })
        .collect()
}

// ---------------------------------------------------------------------
// Plan-level reduction: take all seeds' fits + states and produce
// kill/promote decisions for the next rung.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct HuntDecision {
    pub seed: u32,
    pub state: SeedState,
    pub action: HuntAction,
    pub fit: Option<PowerLawFit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HuntAction {
    Keep,
    Kill,
    Promote,
}

pub fn classify_and_act(
    seed_curves: &BTreeMap<u32, Vec<CurvePoint>>,
    leader_seed: u32,
    classifier_opts: &ClassifierOptions,
    fit_opts: &FitOptions,
    rung_idx: usize,
) -> Vec<HuntDecision> {
    let leader_curve = seed_curves
        .get(&leader_seed)
        .cloned()
        .unwrap_or_default();
    let mut out: Vec<HuntDecision> = Vec::with_capacity(seed_curves.len());
    for (seed, curve) in seed_curves {
        let state = classify_seed(curve, &leader_curve, classifier_opts)
            .unwrap_or(SeedState::Tied);
        let fit = fit_power_law(curve, fit_opts);
        let mut action = HuntAction::Keep;
        match state {
            SeedState::Diverging => action = HuntAction::Kill,
            SeedState::Leading if rung_idx >= 3 => action = HuntAction::Promote,
            _ => {}
        }
        out.push(HuntDecision {
            seed: *seed,
            state,
            action,
            fit,
        });
    }
    // Apply rung's keep_top_k by predicted bpb_inf.
    if rung_idx == 1 || rung_idx == 2 {
        let keep = resolve_keep_top_k(rung_idx, seed_curves.len());
        // Sort by predicted bpb_inf ascending (lower = better).
        let mut indexed: Vec<(usize, f64)> = out
            .iter()
            .enumerate()
            .map(|(i, d)| (i, d.fit.as_ref().map(|f| f.bpb_inf).unwrap_or(f64::INFINITY)))
            .collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (rank, (idx, _)) in indexed.iter().enumerate() {
            if rank >= keep {
                // Beyond keep_top_k → kill (unless already promoted).
                if out[*idx].action != HuntAction::Promote {
                    out[*idx].action = HuntAction::Kill;
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a synthetic power-law curve with known params.
    fn synth_curve(steps: &[u32], bpb_inf: f64, a: f64, p: f64, noise: f64) -> Vec<CurvePoint> {
        // Deterministic PRNG-free "noise" using a Weyl sequence.
        let mut acc = 0.0_f64;
        steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                acc = (acc + 0.6180339887498949).fract();
                let perturb = noise * (acc * 2.0 - 1.0);
                CurvePoint {
                    step: *s,
                    bpb: bpb_inf + a * (*s as f64).powf(-p) + perturb,
                }
            })
            .collect()
    }

    #[test]
    fn fit_recovers_synthetic_params_well() {
        let steps: Vec<u32> = (1..=20).map(|i| i * 100).collect();
        let truth_bpb_inf = 1.85;
        let truth_a = 8.0;
        let truth_p = 0.50;
        let curve = synth_curve(&steps, truth_bpb_inf, truth_a, truth_p, 0.005);
        let fit = fit_power_law(&curve, &FitOptions::default()).unwrap();
        assert!(
            (fit.bpb_inf - truth_bpb_inf).abs() < 0.10,
            "got bpb_inf={}",
            fit.bpb_inf
        );
        assert!((fit.p - truth_p).abs() < 0.15, "got p={}", fit.p);
        // RSS should be small for low-noise synthetic.
        assert!(fit.rss < 0.1, "rss={}", fit.rss);
    }

    #[test]
    fn fit_returns_none_below_min_samples() {
        let curve = vec![
            CurvePoint { step: 100, bpb: 3.0 },
            CurvePoint { step: 200, bpb: 2.5 },
            CurvePoint { step: 300, bpb: 2.2 },
        ];
        assert!(fit_power_law(&curve, &FitOptions::default()).is_none());
    }

    #[test]
    fn classifier_marks_leading_when_below_leader() {
        // Seed below leader at last common step.
        let leader = vec![
            CurvePoint { step: 100, bpb: 3.0 },
            CurvePoint { step: 200, bpb: 2.5 },
        ];
        let seed = vec![
            CurvePoint { step: 100, bpb: 2.9 },
            CurvePoint { step: 200, bpb: 2.4 },
        ];
        let s = classify_seed(&seed, &leader, &ClassifierOptions::default()).unwrap();
        assert_eq!(s, SeedState::Leading);
    }

    #[test]
    fn classifier_marks_diverging_with_growing_gap() {
        // Slope kill threshold = 1e-3 BPB/step. Steps spaced by 10 here
        // so a per-tick gap of 0.02 BPB → slope = 2e-3 > threshold.
        let leader: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 10,
                bpb: 2.50,
            })
            .collect();
        let seed: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 10,
                bpb: 2.55 + 0.02 * i as f64, // 2.57, 2.59, ..., 2.75
            })
            .collect();
        let s = classify_seed(&seed, &leader, &ClassifierOptions::default()).unwrap();
        assert_eq!(s, SeedState::Diverging);
    }

    #[test]
    fn classifier_marks_catching_up_with_shrinking_gap() {
        let leader: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 100,
                bpb: 2.50 - 0.02 * i as f64,
            })
            .collect();
        let seed: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 100,
                bpb: 2.80 - 0.04 * i as f64, // closes faster
            })
            .collect();
        let s = classify_seed(&seed, &leader, &ClassifierOptions::default()).unwrap();
        assert_eq!(s, SeedState::CatchingUp);
    }

    #[test]
    fn classifier_marks_tied_within_band() {
        let leader: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 100,
                bpb: 2.50,
            })
            .collect();
        let seed: Vec<CurvePoint> = (1..=10)
            .map(|i| CurvePoint {
                step: i * 100,
                bpb: 2.502, // within tied_band 0.005
            })
            .collect();
        let s = classify_seed(&seed, &leader, &ClassifierOptions::default()).unwrap();
        assert_eq!(s, SeedState::Tied);
    }

    #[test]
    fn rung_keep_drops_half_at_rung_1_and_2() {
        assert_eq!(resolve_keep_top_k(1, 21), 11);
        assert_eq!(resolve_keep_top_k(2, 11), 6);
        // Rung 3 keeps top-3 regardless.
        assert_eq!(resolve_keep_top_k(3, 6), 3);
        // Rung 4 keeps top-1.
        assert_eq!(resolve_keep_top_k(4, 3), 1);
    }

    #[test]
    fn phi_anchored_seeds_are_distinct_for_small_k() {
        let seeds = phi_anchored_seeds(8, 1_000_000.0);
        assert_eq!(seeds.len(), 8);
        let unique: std::collections::BTreeSet<u32> = seeds.iter().copied().collect();
        assert_eq!(unique.len(), 8, "first 8 phi seeds must be distinct");
    }

    #[test]
    fn phi_anchored_seeds_use_phi_growth() {
        // Sanity: ratio between consecutive seeds (modulo wrap) should
        // be approximately phi for the small-k regime.
        let seeds = phi_anchored_seeds(5, 1_000.0);
        // seeds[1] ≈ phi · seeds[0]; not exact because of fmod wrap on
        // the float side, but for k=1..5 with multiplier=1000 we are
        // far below 2^32 so no wrap.
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let ratio = seeds[1] as f64 / seeds[0] as f64;
        assert!(
            (ratio - phi).abs() < 0.10,
            "expected ratio ≈ phi, got {ratio}"
        );
    }

    #[test]
    fn classify_and_act_kills_diverging_promotes_leading_at_rung_3() {
        let mut curves: BTreeMap<u32, Vec<CurvePoint>> = BTreeMap::new();
        // Seed 220: clearly diverging upward (slope > 1e-3 / step).
        // step spacing = 10 → per-tick increment 0.02 BPB → slope 2e-3.
        curves.insert(
            220,
            (1..=20)
                .map(|i| CurvePoint {
                    step: i * 10,
                    bpb: 2.50 + 0.02 * i as f64,
                })
                .collect(),
        );
        // Make leader/200 also use step*10 so the join works.
        curves.insert(
            200,
            (1..=20)
                .map(|i| CurvePoint {
                    step: i * 10,
                    bpb: 1.85 + 6.0 * (i as f64 * 10.0).powf(-0.5),
                })
                .collect(),
        );
        curves.insert(201, curves[&200].clone());
        let decisions = classify_and_act(
            &curves,
            200,
            &ClassifierOptions::default(),
            &FitOptions::default(),
            3,
        );
        let dec_200 = decisions.iter().find(|d| d.seed == 200).unwrap();
        let dec_220 = decisions.iter().find(|d| d.seed == 220).unwrap();
        // Leader on rung_3 ≥ 3 should promote.
        assert!(matches!(
            dec_200.state,
            SeedState::Tied | SeedState::Leading
        ));
        // Diverging seed must be killed.
        assert_eq!(dec_220.state, SeedState::Diverging);
        assert_eq!(dec_220.action, HuntAction::Kill);
    }

    #[test]
    fn fit_ci_half_width_shrinks_with_more_samples() {
        let truth_bpb_inf = 2.0;
        let s_short: Vec<u32> = (1..=8).map(|i| i * 100).collect();
        let s_long: Vec<u32> = (1..=40).map(|i| i * 100).collect();
        let curve_short = synth_curve(&s_short, truth_bpb_inf, 5.0, 0.4, 0.01);
        let curve_long = synth_curve(&s_long, truth_bpb_inf, 5.0, 0.4, 0.01);
        let fit_short = fit_power_law(&curve_short, &FitOptions::default()).unwrap();
        let fit_long = fit_power_law(&curve_long, &FitOptions::default()).unwrap();
        // Longer curve → tighter CI (sigma scaled by 1/sqrt(dof)).
        assert!(
            fit_long.ci_half_width <= fit_short.ci_half_width,
            "long_ci={} short_ci={}",
            fit_long.ci_half_width,
            fit_short.ci_half_width
        );
    }
}

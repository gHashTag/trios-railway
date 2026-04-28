//! IGLA canonical naming —
//! `IGLA-<MODEL_TYPE>-<NUMBER_FORMAT>-<EXP_ID>[-<TAG>]-rng<SEED>`.
//!
//! `<EXP_ID>` is the **service slot identifier** (not the RNG seed):
//! a monotonically increasing token allocated by the coordinator. The
//! RNG seed used for weight initialisation is recorded separately as
//! `rng<N>` and may repeat across experiments. This INV-12 split fixes
//! the operator's reuse-of-old-service-name footgun: every new
//! experiment gets a fresh `<EXP_ID>` even if it pins the same RNG
//! seed (43/44/45) for reproducibility against the champion record.
//!
//! Champion `<EXP_ID>` slots E0001..E0004 are **immutable** and locked
//! by `CHAMPION_LOCKS`. Any deploy that targets one of those slots is
//! rejected at parse / validate time.
//!
//! Single source of truth shared across:
//! - Rust code (this module)
//! - Railway service names (`tri railway service rename …`)
//! - Neon ledger (`igla_race_trials.config` becomes a string of an
//!   `IglaCanon`)
//! - Leaderboard rendering (`bin/tri-gardener/src/leaderboard.rs`)
//!
//! Three normative rules are enforced at parse time (R5: parsing fails
//! loudly, never returns silently invalid):
//! - **L-R8** (stdout discipline): callers reporting BPB must use the
//!   `BPB=X.XXXX` form. Enforced by `parse_bpb_line`.
//! - **L-R9** (GF16 safe domain): any `IGLA-*-GF16` config must run
//!   with `h >= 256` (Lucas-closure proven safe domain). Enforced by
//!   `IglaCanon::validate_with_capacity`.
//! - **L-METRIC** (no proxy losses as primary): `IGLA-JEPA-T-*` and
//!   `IGLA-NCA-*` must report BPB (NTP CE / ln 2). MSE / reconstruction
//!   proxy is rejected.
//!
//! Anchor: `phi^2 + phi^-2 = 3`.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// All known model architectures used in the IGLA race.
///
/// Variants align with `gHashTag/trios#143` taxonomy: JEPA-T / NCA /
/// PHI / HYBRID / TRINITY3K / TRAIN_V2 / TJEPA / MUON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModelType {
    JepaT,
    Nca,
    Phi,
    Hybrid,
    Trinity3K,
    TrainV2,
    TJepa,
    Muon,
}

impl ModelType {
    pub fn as_canon(&self) -> &'static str {
        match self {
            ModelType::JepaT => "JEPA-T",
            ModelType::Nca => "NCA",
            ModelType::Phi => "PHI",
            ModelType::Hybrid => "HYBRID",
            ModelType::Trinity3K => "TRINITY3K",
            ModelType::TrainV2 => "TRAIN_V2",
            ModelType::TJepa => "TJEPA",
            ModelType::Muon => "MUON",
        }
    }

    /// Architectural BPB ceiling per family — soft floor used by the
    /// gardener's anti-cull logic. `None` means "unknown / open".
    pub fn architectural_floor_bpb(&self) -> Option<f64> {
        match self {
            ModelType::TrainV2 => Some(1.89),
            ModelType::Hybrid => Some(2.19),
            ModelType::Phi => Some(2.21),
            ModelType::TJepa => Some(2.67),
            ModelType::Trinity3K => Some(2.70),
            ModelType::Muon => Some(2.59), // FALSIFIED
            ModelType::JepaT => None,      // grad-flow broken
            ModelType::Nca => None,        // grad-flow broken
        }
    }
}

impl FromStr for ModelType {
    type Err = CanonError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "JEPA-T" => Ok(ModelType::JepaT),
            "NCA" => Ok(ModelType::Nca),
            "PHI" => Ok(ModelType::Phi),
            "HYBRID" => Ok(ModelType::Hybrid),
            "TRINITY3K" => Ok(ModelType::Trinity3K),
            "TRAIN_V2" => Ok(ModelType::TrainV2),
            "TJEPA" => Ok(ModelType::TJepa),
            "MUON" => Ok(ModelType::Muon),
            other => Err(CanonError::UnknownModelType(other.to_string())),
        }
    }
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_canon())
    }
}

/// All number formats considered by the race. Bit layouts mirror
/// `gHashTag/zig-golden-float/docs/whitepaper/gf16_comparison.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NumberFormat {
    Fp32,
    Fp16,
    Bf16,
    Fp8E4M3,
    Fp8E5M2,
    DlFloat,
    Gf8,
    Gf16,
    Gf32,
    Gf64,
    GfTern,
}

impl NumberFormat {
    pub fn as_canon(&self) -> &'static str {
        match self {
            NumberFormat::Fp32 => "FP32",
            NumberFormat::Fp16 => "FP16",
            NumberFormat::Bf16 => "BF16",
            NumberFormat::Fp8E4M3 => "FP8E4M3",
            NumberFormat::Fp8E5M2 => "FP8E5M2",
            NumberFormat::DlFloat => "DLFLOAT",
            NumberFormat::Gf8 => "GF8",
            NumberFormat::Gf16 => "GF16",
            NumberFormat::Gf32 => "GF32",
            NumberFormat::Gf64 => "GF64",
            NumberFormat::GfTern => "GFTERN",
        }
    }

    /// Bit width (S+E+M+padding aware).
    pub fn bits(&self) -> u8 {
        match self {
            NumberFormat::Fp32 | NumberFormat::Gf32 => 32,
            NumberFormat::Fp16
            | NumberFormat::Bf16
            | NumberFormat::DlFloat
            | NumberFormat::Gf16 => 16,
            NumberFormat::Fp8E4M3 | NumberFormat::Fp8E5M2 | NumberFormat::Gf8 => 8,
            NumberFormat::Gf64 => 64,
            NumberFormat::GfTern => 2,
        }
    }
}

impl FromStr for NumberFormat {
    type Err = CanonError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FP32" => Ok(NumberFormat::Fp32),
            "FP16" => Ok(NumberFormat::Fp16),
            "BF16" => Ok(NumberFormat::Bf16),
            "FP8E4M3" => Ok(NumberFormat::Fp8E4M3),
            "FP8E5M2" => Ok(NumberFormat::Fp8E5M2),
            "DLFLOAT" => Ok(NumberFormat::DlFloat),
            "GF8" => Ok(NumberFormat::Gf8),
            "GF16" => Ok(NumberFormat::Gf16),
            "GF32" => Ok(NumberFormat::Gf32),
            "GF64" => Ok(NumberFormat::Gf64),
            "GFTERN" => Ok(NumberFormat::GfTern),
            other => Err(CanonError::UnknownNumberFormat(other.to_string())),
        }
    }
}

impl fmt::Display for NumberFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_canon())
    }
}

/// Parsed canonical name. Always round-trips through `Display`.
///
/// Two shapes are accepted:
///   1. Type-template:   `IGLA-<TYPE>-<FORMAT>` (no `EXP_ID`, no rng)
///   2. Concrete deploy: `IGLA-<TYPE>-<FORMAT>-<EXP_ID>[-<TAG>]-rng<SEED>`
///
/// Plus a legacy seed-only form `IGLA-<TYPE>-<FORMAT>-seed<N>` that is
/// **rejected** by `validate_for_deploy()` (see tripwire #100).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IglaCanon {
    pub model: ModelType,
    pub format: NumberFormat,
    /// Monotonic experiment identifier `E<NNNN>` (zero-padded to 4),
    /// allocated by the coordinator's Neon `exp_id_seq`. `None` for
    /// type-template / legacy names.
    pub exp_id: Option<u32>,
    /// Optional experiment tag, e.g. `WSD`, `BS8`, `GRADFIX`, `EMA10`,
    /// `h512`, `h768`.
    pub tag: Option<String>,
    /// RNG seed used for weight initialisation. Allowed to repeat
    /// across experiments — reproducibility against the champion is
    /// the use-case. `None` for type-template names.
    pub rng: Option<u32>,
    /// Legacy `-seed<N>` suffix kept only for backward parsing of
    /// pre-INV-12 names. Always `None` for newly-allocated names; if
    /// `Some`, `validate_for_deploy()` rejects the canon.
    pub legacy_seed: Option<u32>,
}

impl IglaCanon {
    /// Validate the `(model, format, capacity)` triple against the
    /// normative rules. `capacity` is the model's `h` (hidden width).
    pub fn validate_with_capacity(&self, capacity: u32) -> Result<(), CanonError> {
        // L-R9: GF16 only safe at h >= 256 (Lucas-closure domain).
        if self.format == NumberFormat::Gf16 && capacity < 256 {
            return Err(CanonError::Lr9GfTooSmall {
                h: capacity,
                min: 256,
            });
        }
        // L-METRIC: JEPA-T / NCA must commit to a BPB metric, never an
        // MSE/reconstruction proxy. Encoded here only as a marker — the
        // training-side check is owned by the trainer repo per ADR-0001.
        // Returning Ok at parse time; runtime reporters are gated by
        // `enforce_l_metric()`.
        Ok(())
    }

    /// L-METRIC enforcement helper. Returns `Err` if the architecture
    /// tries to commit to a non-BPB primary loss.
    pub fn enforce_l_metric(&self, primary_loss_kind: &str) -> Result<(), CanonError> {
        let needs_bpb = matches!(self.model, ModelType::JepaT | ModelType::Nca);
        if needs_bpb && primary_loss_kind != "bpb" {
            return Err(CanonError::LMetricNonBpb {
                model: self.model,
                got: primary_loss_kind.to_string(),
            });
        }
        Ok(())
    }

    /// **Tripwire #100** — forbid the legacy `-seed<N>` suffix on any
    /// new deploy. `EXP_ID` + `rng<N>` is the only allowed concrete
    /// form; the bare seed shape is reserved for read-only history.
    ///
    /// **Tripwire #98** — if the parsed `EXP_ID` matches a slot that
    /// `CHAMPION_LOCKS` reports as locked, reject (rotation must spin
    /// the next sequence value, not the champion's slot).
    ///
    /// **Tripwire #99** — require `EXP_ID > current_max` so deploys are
    /// strictly monotonic. Caller passes the current max value
    /// observed in Neon; we reject `<= current_max`.
    pub fn validate_for_deploy(&self, current_max_exp_id: u32) -> Result<(), CanonError> {
        if self.legacy_seed.is_some() {
            return Err(CanonError::NakedSeedInDeployName(self.to_string()));
        }
        let exp_id = self
            .exp_id
            .ok_or_else(|| CanonError::MissingExpId(self.to_string()))?;
        let rng = self
            .rng
            .ok_or_else(|| CanonError::MissingRng(self.to_string()))?;
        let _ = rng; // RNG only validated for presence; values may repeat.
        if let Some(reason) = champion_lock_reason(exp_id) {
            return Err(CanonError::ReusedChampionSlot {
                exp_id,
                reason: reason.to_string(),
            });
        }
        if exp_id <= current_max_exp_id {
            return Err(CanonError::NonMonotonicExpId {
                got: exp_id,
                current_max: current_max_exp_id,
            });
        }
        Ok(())
    }
}

/// Frozen champion slots. Any deploy targeting these `EXP_ID`s is
/// rejected (Tripwire #98). Add a new entry here — in the same PR —
/// the moment a Gate-2 quorum is officially confirmed.
pub const CHAMPION_LOCKS: &[(u32, &str)] = &[
    (1, "E0001 — IGLA-HYBRID-FP32 BPB=2.1919 rng43 (locked 2026-04-27T16:38Z)"),
    (2, "E0002 — IGLA-HYBRID-FP32 BPB=2.1944 rng45 (locked 2026-04-27T16:38Z)"),
    (3, "E0003 — IGLA-HYBRID-FP32 BPB=2.2024 rng44 (locked 2026-04-27T16:38Z)"),
    (4, "E0004 — IGLA-TRAIN_V2-FP32 BPB=1.8921 rng42 (NEW CHAMPION 2026-04-28T05:30Z)"),
];

/// Returns the lock reason if `exp_id` is one of the immutable champion
/// slots, else `None`.
pub fn champion_lock_reason(exp_id: u32) -> Option<&'static str> {
    CHAMPION_LOCKS
        .iter()
        .find(|(e, _)| *e == exp_id)
        .map(|(_, r)| *r)
}

// ---------------------------------------------------------------------
// GATE-0 SMOKE RACE — reserved tag + seed range.
// ---------------------------------------------------------------------

/// The Gate-0 smoke race uses RNG seeds in `[500, 600)` so they never
/// collide with production seeds (Phase-1: 100..299, champion: 42..45).
/// Tripwire #106 enforces this at parse-time.
pub const SMOKE_SEED_RANGE: std::ops::Range<u32> = 500..600;

/// Marker substring that the orchestrator embeds in the tag for any
/// smoke-race deploy. Combined with `SMOKE_SEED_RANGE` this is enough
/// to round-trip identify a smoke service from its canon name alone.
pub const SMOKE_TAG_MARKER: &str = "SMOKE";

/// True iff the canon's `tag` ends with `-SMOKE` (or equals `SMOKE`),
/// i.e. the deploy belongs to a Gate-0 smoke run.
pub fn is_smoke(canon: &IglaCanon) -> bool {
    canon
        .tag
        .as_deref()
        .map(|t| t == SMOKE_TAG_MARKER || t.ends_with("-SMOKE"))
        .unwrap_or(false)
}

/// **Tripwire #106** — smoke deploys must use seeds in the reserved
/// 500..600 window. Production deploys must NOT use that window.
pub fn assert_smoke_seed_range(canon: &IglaCanon) -> Result<(), CanonError> {
    let smoke = is_smoke(canon);
    let rng = canon
        .rng
        .ok_or_else(|| CanonError::MissingRng(canon.to_string()))?;
    let in_range = SMOKE_SEED_RANGE.contains(&rng);
    match (smoke, in_range) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => Err(CanonError::SmokeSeedOutOfRange {
            rng,
            tag: canon.tag.clone().unwrap_or_default(),
        }),
        (false, true) => Err(CanonError::ProductionSeedInSmokeRange {
            rng,
            tag: canon.tag.clone().unwrap_or_default(),
        }),
    }
}

/// Tripwire #101 — kill-before-spin guard. Caller passes the set of
/// service names currently occupying the slot the new deploy targets;
/// if any are still alive AND `force_replace` is `false`, reject.
pub fn assert_kill_before_spin(
    target_name: &str,
    occupants: &[&str],
    force_replace: bool,
) -> Result<(), CanonError> {
    if occupants.is_empty() || force_replace {
        return Ok(());
    }
    Err(CanonError::SlotStillOccupied {
        target: target_name.to_string(),
        occupants: occupants.iter().map(|s| s.to_string()).collect(),
    })
}

impl fmt::Display for IglaCanon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IGLA-{}-{}", self.model, self.format)?;
        if let Some(exp) = self.exp_id {
            write!(f, "-E{:04}", exp)?;
        }
        if let Some(tag) = &self.tag {
            write!(f, "-{}", tag)?;
        }
        if let Some(rng) = self.rng {
            write!(f, "-rng{}", rng)?;
        }
        if let Some(seed) = self.legacy_seed {
            write!(f, "-seed{}", seed)?;
        }
        Ok(())
    }
}

impl FromStr for IglaCanon {
    type Err = CanonError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();

        // Peel suffixes in reverse: legacy -seedN, then -rngN.
        let (head, legacy_seed) = match trimmed.rsplit_once("-seed") {
            Some((h, n)) => match n.parse::<u32>() {
                Ok(n) => (h.to_string(), Some(n)),
                Err(_) => (trimmed.to_string(), None),
            },
            None => (trimmed.to_string(), None),
        };
        let (head, rng) = match head.rsplit_once("-rng") {
            Some((h, n)) => match n.parse::<u32>() {
                Ok(n) => (h.to_string(), Some(n)),
                Err(_) => (head, None),
            },
            None => (head, None),
        };

        let body = head
            .strip_prefix("IGLA-")
            .ok_or_else(|| CanonError::MissingIglaPrefix(trimmed.to_string()))?
            .to_string();

        let model_candidates = [
            "TRINITY3K",
            "TRAIN_V2",
            "JEPA-T",
            "HYBRID",
            "TJEPA",
            "MUON",
            "PHI",
            "NCA",
        ];
        let mut model: Option<ModelType> = None;
        let mut rest: String = String::new();
        for candidate in model_candidates {
            if let Some(after) = body.strip_prefix(candidate) {
                if after.is_empty() || after.starts_with('-') {
                    model = Some(candidate.parse::<ModelType>()?);
                    rest = after.strip_prefix('-').unwrap_or(after).to_string();
                    break;
                }
            }
        }
        let model = model.ok_or(CanonError::UnknownModelType(body.clone()))?;

        if rest.is_empty() {
            return Err(CanonError::MissingFormat(trimmed.to_string()));
        }
        // First component is the format; the rest may be `<EXP_ID>` and/or `<TAG>`.
        let (format_str, mut after_format) = match rest.split_once('-') {
            Some((f, t)) => (f.to_string(), t.to_string()),
            None => (rest, String::new()),
        };
        let format = format_str.parse::<NumberFormat>()?;

        // Try to peel `E<NNNN>` from the front of `after_format`.
        let mut exp_id: Option<u32> = None;
        if !after_format.is_empty() {
            let (head_token, tail) = match after_format.split_once('-') {
                Some((h, t)) => (h.to_string(), t.to_string()),
                None => (after_format.clone(), String::new()),
            };
            if let Some(num_part) = head_token.strip_prefix('E') {
                if let Ok(n) = num_part.parse::<u32>() {
                    exp_id = Some(n);
                    after_format = tail;
                }
            }
        }
        let tag = if after_format.is_empty() {
            None
        } else {
            Some(after_format)
        };

        Ok(IglaCanon {
            model,
            format,
            exp_id,
            tag,
            rng,
            legacy_seed,
        })
    }
}

/// L-R8 stdout discipline: parse a single line of trainer stdout and
/// return the BPB if it matches the canonical `BPB=X.XXXX` form. Any
/// other shape returns `None` — caller must skip without claiming a
/// reading.
pub fn parse_bpb_line(line: &str) -> Option<f64> {
    use regex::Regex;
    // Lazy-static would be nicer, but the gardener already paid the
    // regex compile cost in `bpb_source.rs`. One-shot here is fine.
    let re = Regex::new(r"\bBPB=(\d+\.\d{4})\b").ok()?;
    let caps = re.captures(line)?;
    caps.get(1)?.as_str().parse::<f64>().ok()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonError {
    #[error("name does not start with `IGLA-`: {0:?}")]
    MissingIglaPrefix(String),
    #[error("unknown <MODEL_TYPE>: {0:?}")]
    UnknownModelType(String),
    #[error("missing <NUMBER_FORMAT> after model: {0:?}")]
    MissingFormat(String),
    #[error("unknown <NUMBER_FORMAT>: {0:?}")]
    UnknownNumberFormat(String),
    #[error("L-R9 violation: GF16 requires h >= {min}, got h={h}")]
    Lr9GfTooSmall { h: u32, min: u32 },
    #[error("L-METRIC violation: {model} must report BPB, got {got:?}")]
    LMetricNonBpb { model: ModelType, got: String },
    #[error("INV-12 #98: cannot reuse champion slot E{exp_id:04}: {reason}")]
    ReusedChampionSlot { exp_id: u32, reason: String },
    #[error("INV-12 #99: EXP_ID must be strictly monotonic; got E{got:04}, current max E{current_max:04}")]
    NonMonotonicExpId { got: u32, current_max: u32 },
    #[error("INV-12 #100: legacy `-seed<N>` suffix forbidden on new deploy: {0:?}")]
    NakedSeedInDeployName(String),
    #[error("INV-12 #101: slot still occupied by {occupants:?}; pass --force-replace or kill first; target={target:?}")]
    SlotStillOccupied {
        target: String,
        occupants: Vec<String>,
    },
    #[error("INV-12: deploy name missing <EXP_ID>: {0:?}")]
    MissingExpId(String),
    #[error("INV-12: deploy name missing <rngN>: {0:?}")]
    MissingRng(String),
    #[error("INV-12 #106: smoke deploy with rng={rng} outside reserved range 500..600 (tag={tag:?})")]
    SmokeSeedOutOfRange { rng: u32, tag: String },
    #[error("INV-12 #106: production deploy with rng={rng} collides with reserved smoke range 500..600 (tag={tag:?})")]
    ProductionSeedInSmokeRange { rng: u32, tag: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// User's canonical name list from the operator brief, verbatim.
    /// Each entry must round-trip through parse + Display.
    const CANONICAL_NAMES: &[&str] = &[
        "IGLA-JEPA-T-FP32",
        "IGLA-JEPA-T-GF16",
        "IGLA-JEPA-T-BF16",
        "IGLA-JEPA-T-FP16",
        "IGLA-JEPA-T-DLFLOAT",
        "IGLA-JEPA-T-FP8E4M3",
        "IGLA-JEPA-T-FP8E5M2",
        "IGLA-JEPA-T-GF8",
        "IGLA-JEPA-T-GF32",
        "IGLA-JEPA-T-GF64",
        "IGLA-JEPA-T-GFTERN",
        "IGLA-NCA-FP32",
        "IGLA-NCA-GF16",
        "IGLA-NCA-BF16",
        "IGLA-NCA-FP16",
        "IGLA-NCA-DLFLOAT",
        "IGLA-NCA-GFTERN",
        "IGLA-PHI-FP32",
        "IGLA-PHI-GF16",
        "IGLA-PHI-BF16",
        "IGLA-PHI-DLFLOAT",
        "IGLA-PHI-GF8",
        "IGLA-HYBRID-FP32",
        "IGLA-HYBRID-GF16",
        "IGLA-HYBRID-BF16",
        "IGLA-HYBRID-FP16",
        "IGLA-HYBRID-DLFLOAT",
        "IGLA-HYBRID-FP8E4M3",
        "IGLA-HYBRID-FP8E5M2",
        "IGLA-TRINITY3K-FP32",
        "IGLA-TRINITY3K-GF16",
        "IGLA-TRAIN_V2-FP32",
        "IGLA-TRAIN_V2-GF16",
        "IGLA-TRAIN_V2-BF16",
        "IGLA-TRAIN_V2-FP16",
        "IGLA-TRAIN_V2-DLFLOAT",
        "IGLA-TRAIN_V2-GF32",
        "IGLA-TRAIN_V2-GF64",
        "IGLA-TJEPA-FP32",
        "IGLA-TJEPA-GF16",
        "IGLA-MUON-FP32",
        "IGLA-MUON-GF16",
    ];

    #[test]
    fn every_canonical_name_round_trips() {
        for name in CANONICAL_NAMES {
            let parsed: IglaCanon = name.parse().unwrap_or_else(|e| {
                panic!("failed to parse {name:?}: {e}");
            });
            let rendered = parsed.to_string();
            assert_eq!(&rendered, name, "round-trip mismatch on {name:?}");
        }
    }

    #[test]
    fn parse_legacy_seed_suffix_is_preserved_for_history() {
        let n: IglaCanon = "IGLA-TRAIN_V2-FP32-seed42".parse().unwrap();
        assert_eq!(n.model, ModelType::TrainV2);
        assert_eq!(n.format, NumberFormat::Fp32);
        assert_eq!(n.tag, None);
        assert_eq!(n.exp_id, None);
        assert_eq!(n.rng, None);
        assert_eq!(n.legacy_seed, Some(42));
        assert_eq!(n.to_string(), "IGLA-TRAIN_V2-FP32-seed42");
    }

    #[test]
    fn parse_full_inv12_form() {
        let n: IglaCanon = "IGLA-HYBRID-FP32-E0042-WSD-rng201".parse().unwrap();
        assert_eq!(n.model, ModelType::Hybrid);
        assert_eq!(n.format, NumberFormat::Fp32);
        assert_eq!(n.exp_id, Some(42));
        assert_eq!(n.tag.as_deref(), Some("WSD"));
        assert_eq!(n.rng, Some(201));
        assert_eq!(n.legacy_seed, None);
        assert_eq!(n.to_string(), "IGLA-HYBRID-FP32-E0042-WSD-rng201");
    }

    #[test]
    fn parse_inv12_form_without_tag() {
        let n: IglaCanon = "IGLA-TRAIN_V2-FP32-E0042-rng43".parse().unwrap();
        assert_eq!(n.exp_id, Some(42));
        assert_eq!(n.tag, None);
        assert_eq!(n.rng, Some(43));
        assert_eq!(n.to_string(), "IGLA-TRAIN_V2-FP32-E0042-rng43");
    }

    #[test]
    fn parse_jepa_t_keeps_internal_hyphen() {
        // `JEPA-T` contains a hyphen as part of the canonical model
        // name; the parser must not greedily eat it.
        let n: IglaCanon = "IGLA-JEPA-T-GF16".parse().unwrap();
        assert_eq!(n.model, ModelType::JepaT);
        assert_eq!(n.format, NumberFormat::Gf16);
    }

    #[test]
    fn unknown_prefix_is_rejected() {
        assert!(matches!(
            "DOGE-TRAIN_V2-FP32".parse::<IglaCanon>(),
            Err(CanonError::MissingIglaPrefix(_))
        ));
    }

    #[test]
    fn unknown_format_is_rejected() {
        let err = "IGLA-HYBRID-INT4".parse::<IglaCanon>().unwrap_err();
        assert!(matches!(err, CanonError::UnknownNumberFormat(_)));
    }

    #[test]
    fn unknown_model_is_rejected() {
        let err = "IGLA-RNN-FP32".parse::<IglaCanon>().unwrap_err();
        assert!(matches!(err, CanonError::UnknownModelType(_)));
    }

    #[test]
    fn lr9_rejects_gf16_below_256() {
        let n: IglaCanon = "IGLA-TRAIN_V2-GF16".parse().unwrap();
        let err = n.validate_with_capacity(128).unwrap_err();
        assert!(matches!(
            err,
            CanonError::Lr9GfTooSmall { h: 128, min: 256 }
        ));
    }

    #[test]
    fn lr9_accepts_gf16_at_256_and_above() {
        let n: IglaCanon = "IGLA-TRAIN_V2-GF16".parse().unwrap();
        assert!(n.validate_with_capacity(256).is_ok());
        assert!(n.validate_with_capacity(1024).is_ok());
    }

    #[test]
    fn lr9_does_not_apply_to_non_gf16() {
        let n: IglaCanon = "IGLA-HYBRID-FP32".parse().unwrap();
        assert!(n.validate_with_capacity(27).is_ok());
    }

    #[test]
    fn l_metric_requires_bpb_for_jepa_t_and_nca() {
        let jepa: IglaCanon = "IGLA-JEPA-T-FP32".parse().unwrap();
        assert!(jepa.enforce_l_metric("mse").is_err());
        assert!(jepa.enforce_l_metric("bpb").is_ok());

        let nca: IglaCanon = "IGLA-NCA-FP32".parse().unwrap();
        assert!(nca.enforce_l_metric("reconstruction").is_err());
        assert!(nca.enforce_l_metric("bpb").is_ok());
    }

    #[test]
    fn l_metric_does_not_apply_to_train_v2() {
        let n: IglaCanon = "IGLA-TRAIN_V2-FP32".parse().unwrap();
        // Even an "mse" primary on TRAIN_V2 is fine here — the rule is
        // scoped to JEPA-T / NCA. The trainer-side training contract
        // is the layer that picks the right NTP CE.
        assert!(n.enforce_l_metric("mse").is_ok());
    }

    // -----------------------------------------------------------------
    // INV-12 tripwires (#98..#101)
    // -----------------------------------------------------------------

    /// Tripwire #98: deploys to a champion-locked EXP_ID are rejected.
    #[test]
    fn tripwire_98_reject_reused_service_name() {
        // E0001 is the champion lock from CHAMPION_LOCKS.
        let n: IglaCanon = "IGLA-HYBRID-FP32-E0001-WSD-rng43".parse().unwrap();
        let err = n.validate_for_deploy(0).unwrap_err();
        assert!(
            matches!(err, CanonError::ReusedChampionSlot { exp_id: 1, .. }),
            "expected ReusedChampionSlot, got {err}"
        );
    }

    /// Tripwire #99: EXP_ID must strictly exceed the current max.
    #[test]
    fn tripwire_99_require_monotonic_exp_id() {
        let n: IglaCanon = "IGLA-HYBRID-FP32-E0010-WSD-rng43".parse().unwrap();
        // current_max = 41 — E0010 is in the past, must reject.
        let err = n.validate_for_deploy(41).unwrap_err();
        assert!(
            matches!(
                err,
                CanonError::NonMonotonicExpId {
                    got: 10,
                    current_max: 41
                }
            ),
            "expected NonMonotonicExpId, got {err}"
        );
        // E0042 strictly > current_max=41 — must accept.
        let n2: IglaCanon = "IGLA-HYBRID-FP32-E0042-WSD-rng43".parse().unwrap();
        assert!(n2.validate_for_deploy(41).is_ok());
    }

    /// Tripwire #100: bare `-seed<N>` shape is forbidden on deploy.
    #[test]
    fn tripwire_100_forbid_naked_seed_in_name() {
        // Legacy parsing succeeds (we keep it for read-only history),
        // but validate_for_deploy rejects.
        let n: IglaCanon = "IGLA-HYBRID-FP32-seed43".parse().unwrap();
        let err = n.validate_for_deploy(0).unwrap_err();
        assert!(
            matches!(err, CanonError::NakedSeedInDeployName(_)),
            "expected NakedSeedInDeployName, got {err}"
        );
    }

    /// Tripwire #101: a still-occupied slot blocks deploy unless
    /// `--force-replace` is passed.
    #[test]
    fn tripwire_101_kill_before_spin() {
        let target = "IGLA-HYBRID-FP32-E0042-WSD-rng43";
        let occupants = ["trios-train-seed-210-L1-attnbw"];
        let err = assert_kill_before_spin(target, &occupants, false).unwrap_err();
        assert!(
            matches!(err, CanonError::SlotStillOccupied { .. }),
            "expected SlotStillOccupied, got {err}"
        );
        // Same call with force_replace=true — must accept.
        assert!(assert_kill_before_spin(target, &occupants, true).is_ok());
        // Empty occupants list — must accept.
        assert!(assert_kill_before_spin(target, &[], false).is_ok());
    }

    /// Confirm the four champion locks shipped in CHAMPION_LOCKS.
    #[test]
    fn champion_locks_cover_e0001_through_e0004() {
        for slot in 1..=4 {
            assert!(
                champion_lock_reason(slot).is_some(),
                "E{slot:04} should be locked"
            );
        }
        assert!(champion_lock_reason(5).is_none());
        assert!(champion_lock_reason(42).is_none());
    }

    /// Regression: non-deploy validations (L-R9, L-METRIC) still work
    /// on type-template names without EXP_ID.
    #[test]
    fn type_template_name_skips_inv12_checks() {
        let n: IglaCanon = "IGLA-TRAIN_V2-GF16".parse().unwrap();
        assert_eq!(n.exp_id, None);
        assert_eq!(n.rng, None);
        // L-R9 still works
        assert!(n.validate_with_capacity(256).is_ok());
        // But validate_for_deploy refuses (missing exp_id)
        assert!(matches!(
            n.validate_for_deploy(0).unwrap_err(),
            CanonError::MissingExpId(_)
        ));
    }

    // -----------------------------------------------------------------
    // GATE-0 SMOKE RACE tripwire #106
    // -----------------------------------------------------------------

    #[test]
    fn smoke_marker_detection() {
        let smoke: IglaCanon = "IGLA-HYBRID-FP32-E0500-WSD-SMOKE-rng500".parse().unwrap();
        assert!(is_smoke(&smoke));
        let prod: IglaCanon = "IGLA-HYBRID-FP32-E0042-WSD-rng201".parse().unwrap();
        assert!(!is_smoke(&prod));
    }

    #[test]
    fn tripwire_106_smoke_seed_must_be_in_500_600_range() {
        let bad: IglaCanon = "IGLA-HYBRID-FP32-E0500-WSD-SMOKE-rng201".parse().unwrap();
        assert!(matches!(
            assert_smoke_seed_range(&bad).unwrap_err(),
            CanonError::SmokeSeedOutOfRange { rng: 201, .. }
        ));
        let good: IglaCanon = "IGLA-HYBRID-FP32-E0500-WSD-SMOKE-rng500".parse().unwrap();
        assert!(assert_smoke_seed_range(&good).is_ok());
    }

    #[test]
    fn tripwire_106_production_must_avoid_smoke_seed_range() {
        // Even if the tag is plain WSD (production), rng=550 is in the
        // smoke window and must be rejected to keep the windows disjoint.
        let bad: IglaCanon = "IGLA-HYBRID-FP32-E0042-WSD-rng550".parse().unwrap();
        assert!(matches!(
            assert_smoke_seed_range(&bad).unwrap_err(),
            CanonError::ProductionSeedInSmokeRange { rng: 550, .. }
        ));
        let good: IglaCanon = "IGLA-HYBRID-FP32-E0042-WSD-rng201".parse().unwrap();
        assert!(assert_smoke_seed_range(&good).is_ok());
    }

    #[test]
    fn smoke_seed_range_is_500_to_599_inclusive() {
        assert!(SMOKE_SEED_RANGE.contains(&500));
        assert!(SMOKE_SEED_RANGE.contains(&599));
        assert!(!SMOKE_SEED_RANGE.contains(&499));
        assert!(!SMOKE_SEED_RANGE.contains(&600));
    }

    #[test]
    fn architectural_floor_train_v2_below_hybrid() {
        // Sanity: the new champion's family floor (1.89) sits below the
        // prior champion's family floor (2.19). Locks the architectural
        // pivot recorded in docs/POSTMORTEM_GATE2_LOCAL_WIN.md.
        let trainv2 = ModelType::TrainV2.architectural_floor_bpb().unwrap();
        let hybrid = ModelType::Hybrid.architectural_floor_bpb().unwrap();
        assert!(trainv2 < hybrid);
    }

    #[test]
    fn parse_bpb_line_canonical_form() {
        assert_eq!(parse_bpb_line("BPB=1.8921"), Some(1.8921));
        assert_eq!(
            parse_bpb_line("info: step=94500 BPB=1.8921 lr=0.002"),
            Some(1.8921)
        );
    }

    #[test]
    fn parse_bpb_line_rejects_non_canonical() {
        // Three-decimal form is rejected — L-R8 demands four digits.
        assert_eq!(parse_bpb_line("BPB=1.892"), None);
        // Lowercase / spaced are also rejected.
        assert_eq!(parse_bpb_line("bpb 1.8921"), None);
        assert_eq!(parse_bpb_line("loss=2.1919"), None);
    }
}

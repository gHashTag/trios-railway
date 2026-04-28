//! IGLA canonical naming — `IGLA-<MODEL_TYPE>-<NUMBER_FORMAT>[-<TAG>]-seed<N>`.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IglaCanon {
    pub model: ModelType,
    pub format: NumberFormat,
    /// Optional experiment tag, e.g. `WSD`, `BS8`, `GRADFIX`, `EMA10`,
    /// `h512`, `h768`. Empty for the bare-bones `IGLA-<TYPE>-<NUM>`
    /// shape used by ad-hoc benchmarks.
    pub tag: Option<String>,
    /// Optional `seed<N>` suffix; `None` for type-template names like
    /// `IGLA-JEPA-T-FP32`.
    pub seed: Option<u32>,
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
}

impl fmt::Display for IglaCanon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IGLA-{}-{}", self.model, self.format)?;
        if let Some(tag) = &self.tag {
            write!(f, "-{}", tag)?;
        }
        if let Some(seed) = self.seed {
            write!(f, "-seed{}", seed)?;
        }
        Ok(())
    }
}

impl FromStr for IglaCanon {
    type Err = CanonError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        // Split a trailing `seedN` if present.
        let (head, seed) = match trimmed.rsplit_once("-seed") {
            Some((h, n)) => match n.parse::<u32>() {
                Ok(n) => (h.to_string(), Some(n)),
                Err(_) => (trimmed.to_string(), None),
            },
            None => (trimmed.to_string(), None),
        };

        // Must start with IGLA-
        let body = head
            .strip_prefix("IGLA-")
            .ok_or_else(|| CanonError::MissingIglaPrefix(trimmed.to_string()))?;

        // Strategy: ModelType is one of a closed set; try each known
        // canonical model name as a prefix on `body` (longest first so
        // `JEPA-T` wins over a hypothetical `JEPA`). The remainder
        // becomes `<FORMAT>` or `<FORMAT>-<TAG>`.
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
        let mut rest: &str = "";
        for candidate in model_candidates {
            if let Some(after) = body.strip_prefix(candidate) {
                if after.is_empty() || after.starts_with('-') {
                    model = Some(candidate.parse::<ModelType>()?);
                    rest = after.strip_prefix('-').unwrap_or(after);
                    break;
                }
            }
        }
        let model = model.ok_or_else(|| CanonError::UnknownModelType(body.to_string()))?;

        // `rest` is `FORMAT` or `FORMAT-TAG-...` or empty.
        if rest.is_empty() {
            return Err(CanonError::MissingFormat(trimmed.to_string()));
        }
        let (format_str, tag) = match rest.split_once('-') {
            Some((f, t)) => (f.to_string(), Some(t.to_string())),
            None => (rest.to_string(), None),
        };
        let format = format_str.parse::<NumberFormat>()?;

        Ok(IglaCanon {
            model,
            format,
            tag,
            seed,
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
    LMetricNonBpb {
        model: ModelType,
        got: String,
    },
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
    fn parse_with_seed_suffix() {
        let n: IglaCanon = "IGLA-TRAIN_V2-FP32-seed42".parse().unwrap();
        assert_eq!(n.model, ModelType::TrainV2);
        assert_eq!(n.format, NumberFormat::Fp32);
        assert_eq!(n.tag, None);
        assert_eq!(n.seed, Some(42));
        assert_eq!(n.to_string(), "IGLA-TRAIN_V2-FP32-seed42");
    }

    #[test]
    fn parse_with_tag_and_seed() {
        let n: IglaCanon = "IGLA-HYBRID-FP32-WSD-seed201".parse().unwrap();
        assert_eq!(n.model, ModelType::Hybrid);
        assert_eq!(n.format, NumberFormat::Fp32);
        assert_eq!(n.tag.as_deref(), Some("WSD"));
        assert_eq!(n.seed, Some(201));
        assert_eq!(n.to_string(), "IGLA-HYBRID-FP32-WSD-seed201");
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

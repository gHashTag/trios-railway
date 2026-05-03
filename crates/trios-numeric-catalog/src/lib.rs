//! `trios-numeric-catalog` — the 64 canonical numeric formats swept by WAVE-GF-001.
//!
//! Single source of truth. Every experiment, leaderboard, and schedule must resolve
//! its `format_canon` token against [`NumericFormat::token`]. Adding a new format
//! requires bumping the `ALL` array (compile-time check) and filing a PR against
//! this crate — catalog changes are versioned, auditable, and CI-gated.
//!
//! Tiers (per operator plan 2026-05-03):
//!
//! | Tier | Meaning | Example |
//! |------|---------|---------|
//! | T1   | runnable today — kernel + trainer integration ship in GHCR image | `binary32`, `GF16` |
//! | T2   | near-runnable — kernel landing via BENCH-012 | `binary64`, `TF32`, `FP8-E4M3` |
//! | T3   | research — spec-only, experimental kernel | `Posit32`, `MXFP8`, `LNS` |
//! | T4   | exotic wide (>64 bit) | `binary128`, `quad-double`, `FP80` |
//! | T5   | micro (sub-byte) | `FP4-E2M1`, `GF4`, `INT4` |
//! | T6   | decimal | `decimal32`, `decimal64`, `decimal128` |
//! | T7   | historical / archival (read-only) | `VAX-F`, `Cray-float`, `IBM-HFP` |
//! | T8   | unum family | `Unum-I`, `Unum-II` |
//! | T9   | fixed / encoded | `Q-format`, `BCD`, `minifloat` |
//!
//! References:
//! - [trios#143](https://github.com/gHashTag/trios/issues/143) IGLA RACE master
//! - [trios-trainer-igla#93](https://github.com/gHashTag/trios-trainer-igla/issues/93) canon-name spec
//! - [zig-golden-float#69](https://github.com/gHashTag/zig-golden-float/pull/69) Universal Numeric-Format Catalog whitepaper §12

use serde::{Deserialize, Serialize};

/// Tier classification. Drives scheduling priority + runnability gating.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    T1Runnable,
    T2NearRunnable,
    T3Research,
    T4ExoticWide,
    T5Micro,
    T6Decimal,
    T7Historical,
    T8Unum,
    T9FixedEncoded,
}

impl Tier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Tier::T1Runnable => "T1",
            Tier::T2NearRunnable => "T2",
            Tier::T3Research => "T3",
            Tier::T4ExoticWide => "T4",
            Tier::T5Micro => "T5",
            Tier::T6Decimal => "T6",
            Tier::T7Historical => "T7",
            Tier::T8Unum => "T8",
            Tier::T9FixedEncoded => "T9",
        }
    }
}

/// Storage container for a packed format.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Storage {
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    /// Two-word (128-bit pair, composite).
    DoubleU64,
    /// Four-word (256-bit pair, composite).
    QuadU64,
    /// Variable-length (Unum I/II, tapered).
    Variable,
    /// Block of N × Ubytes (block-FP, MXFP, shared-exp).
    Block,
}

/// One entry in the 63-format catalog.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormatEntry {
    pub token: &'static str,
    pub tier: Tier,
    pub bits: u32,
    pub exp_bits: Option<u32>,
    pub mant_bits: Option<u32>,
    pub storage: Storage,
    /// True iff the format has a working kernel in the current GHCR trainer image.
    pub runnable: bool,
}

impl FormatEntry {
    pub const fn canonical_prefix_example(&self) -> &'static str {
        self.token
    }
}

/// The full catalog — 63 entries, operator instruction 2026-05-02.
///
/// Order is preserved from the operator's original list for auditability.
/// Any additions/removals must bump this array and land in a PR.
#[rustfmt::skip]
pub const ALL: &[FormatEntry] = &[
    // ── IEEE 754 binary (T1 runnable + T4 exotic wide) ──────────────────────
    FormatEntry { token: "binary16",  tier: Tier::T1Runnable,    bits: 16,  exp_bits: Some(5),  mant_bits: Some(10), storage: Storage::U16,        runnable: true  },
    FormatEntry { token: "binary32",  tier: Tier::T1Runnable,    bits: 32,  exp_bits: Some(8),  mant_bits: Some(23), storage: Storage::U32,        runnable: true  },
    FormatEntry { token: "binary64",  tier: Tier::T2NearRunnable,bits: 64,  exp_bits: Some(11), mant_bits: Some(52), storage: Storage::U64,        runnable: false },
    FormatEntry { token: "binary128", tier: Tier::T4ExoticWide,  bits: 128, exp_bits: Some(15), mant_bits: Some(112),storage: Storage::U128,       runnable: false },
    FormatEntry { token: "binary256", tier: Tier::T4ExoticWide,  bits: 256, exp_bits: Some(19), mant_bits: Some(236),storage: Storage::U256,       runnable: false },
    // ── IEEE 754 decimal (T6) ────────────────────────────────────────────────
    FormatEntry { token: "decimal32", tier: Tier::T6Decimal,     bits: 32,  exp_bits: None,     mant_bits: None,     storage: Storage::U32,        runnable: false },
    FormatEntry { token: "decimal64", tier: Tier::T6Decimal,     bits: 64,  exp_bits: None,     mant_bits: None,     storage: Storage::U64,        runnable: false },
    FormatEntry { token: "decimal128",tier: Tier::T6Decimal,     bits: 128, exp_bits: None,     mant_bits: None,     storage: Storage::U128,       runnable: false },
    // ── Extended / composite (T4) ────────────────────────────────────────────
    FormatEntry { token: "FP80",         tier: Tier::T4ExoticWide, bits: 80,  exp_bits: Some(15), mant_bits: Some(64), storage: Storage::U128,     runnable: false },
    FormatEntry { token: "double-double",tier: Tier::T4ExoticWide, bits: 128, exp_bits: None,     mant_bits: None,     storage: Storage::DoubleU64,runnable: false },
    FormatEntry { token: "quad-double",  tier: Tier::T4ExoticWide, bits: 256, exp_bits: None,     mant_bits: None,     storage: Storage::QuadU64,  runnable: false },
    // ── Reduced-precision training (T1 runnable / T2) ────────────────────────
    FormatEntry { token: "bfloat16", tier: Tier::T1Runnable,    bits: 16, exp_bits: Some(8),  mant_bits: Some(7), storage: Storage::U16, runnable: true  },
    FormatEntry { token: "TF32",     tier: Tier::T2NearRunnable,bits: 19, exp_bits: Some(8),  mant_bits: Some(10),storage: Storage::U32, runnable: false },
    // ── FP8 / FP6 / FP4 microfloats (T2/T5) ──────────────────────────────────
    FormatEntry { token: "FP8-E4M3", tier: Tier::T2NearRunnable, bits: 8, exp_bits: Some(4), mant_bits: Some(3), storage: Storage::U8,  runnable: false },
    FormatEntry { token: "FP8-E5M2", tier: Tier::T2NearRunnable, bits: 8, exp_bits: Some(5), mant_bits: Some(2), storage: Storage::U8,  runnable: false },
    FormatEntry { token: "FP6-E3M2", tier: Tier::T5Micro,        bits: 6, exp_bits: Some(3), mant_bits: Some(2), storage: Storage::U8,  runnable: false },
    FormatEntry { token: "FP6-E2M3", tier: Tier::T5Micro,        bits: 6, exp_bits: Some(2), mant_bits: Some(3), storage: Storage::U8,  runnable: false },
    FormatEntry { token: "FP4-E2M1", tier: Tier::T5Micro,        bits: 4, exp_bits: Some(2), mant_bits: Some(1), storage: Storage::U8,  runnable: false },
    // ── Block-scaled microfloats (T3) ────────────────────────────────────────
    FormatEntry { token: "MXFP8", tier: Tier::T3Research, bits: 8, exp_bits: Some(4), mant_bits: Some(3), storage: Storage::Block, runnable: false },
    FormatEntry { token: "MXFP6", tier: Tier::T3Research, bits: 6, exp_bits: Some(3), mant_bits: Some(2), storage: Storage::Block, runnable: false },
    FormatEntry { token: "MXFP4", tier: Tier::T3Research, bits: 4, exp_bits: Some(2), mant_bits: Some(1), storage: Storage::Block, runnable: false },
    FormatEntry { token: "NF4",   tier: Tier::T3Research, bits: 4, exp_bits: None,    mant_bits: None,    storage: Storage::U8,    runnable: false },
    FormatEntry { token: "AFP",   tier: Tier::T3Research, bits: 0, exp_bits: None,    mant_bits: None,    storage: Storage::Block, runnable: false },
    // ── Posit (T3) ───────────────────────────────────────────────────────────
    FormatEntry { token: "Posit8",  tier: Tier::T3Research, bits: 8,  exp_bits: None, mant_bits: None, storage: Storage::U8,  runnable: false },
    FormatEntry { token: "Posit16", tier: Tier::T3Research, bits: 16, exp_bits: None, mant_bits: None, storage: Storage::U16, runnable: false },
    FormatEntry { token: "Posit32", tier: Tier::T3Research, bits: 32, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    FormatEntry { token: "Posit64", tier: Tier::T3Research, bits: 64, exp_bits: None, mant_bits: None, storage: Storage::U64, runnable: false },
    // ── LNS (T3) ─────────────────────────────────────────────────────────────
    FormatEntry { token: "LNS", tier: Tier::T3Research, bits: 32, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    // ── GoldenFloat family (T1 runnable / T2 / T5) ───────────────────────────
    FormatEntry { token: "GF4",  tier: Tier::T5Micro,        bits: 4,  exp_bits: None, mant_bits: None, storage: Storage::U8,  runnable: false },
    FormatEntry { token: "GF8",  tier: Tier::T2NearRunnable, bits: 8,  exp_bits: None, mant_bits: None, storage: Storage::U8,  runnable: false },
    FormatEntry { token: "GF12", tier: Tier::T5Micro,        bits: 12, exp_bits: None, mant_bits: None, storage: Storage::U16, runnable: false },
    FormatEntry { token: "GF16", tier: Tier::T1Runnable,    bits: 16, exp_bits: None, mant_bits: None, storage: Storage::U16, runnable: true  },
    FormatEntry { token: "GF20", tier: Tier::T5Micro,        bits: 20, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    FormatEntry { token: "GF24", tier: Tier::T5Micro,        bits: 24, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    FormatEntry { token: "GF32", tier: Tier::T2NearRunnable, bits: 32, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    FormatEntry { token: "GF64", tier: Tier::T5Micro,        bits: 64, exp_bits: None, mant_bits: None, storage: Storage::U64, runnable: false },
    // ── Signed integers (T2 / T4 / T5) ───────────────────────────────────────
    FormatEntry { token: "INT4",  tier: Tier::T5Micro,        bits: 4,   exp_bits: None, mant_bits: None, storage: Storage::U8,   runnable: false },
    FormatEntry { token: "INT8",  tier: Tier::T2NearRunnable, bits: 8,   exp_bits: None, mant_bits: None, storage: Storage::U8,   runnable: false },
    FormatEntry { token: "INT16", tier: Tier::T2NearRunnable, bits: 16,  exp_bits: None, mant_bits: None, storage: Storage::U16,  runnable: false },
    FormatEntry { token: "INT32", tier: Tier::T2NearRunnable, bits: 32,  exp_bits: None, mant_bits: None, storage: Storage::U32,  runnable: false },
    FormatEntry { token: "INT64", tier: Tier::T9FixedEncoded, bits: 64,  exp_bits: None, mant_bits: None, storage: Storage::U64,  runnable: false },
    FormatEntry { token: "INT128",tier: Tier::T4ExoticWide,   bits: 128, exp_bits: None, mant_bits: None, storage: Storage::U128, runnable: false },
    // ── Unsigned integers (T2 / T4 / T5) ─────────────────────────────────────
    FormatEntry { token: "UINT4",  tier: Tier::T5Micro,        bits: 4,   exp_bits: None, mant_bits: None, storage: Storage::U8,   runnable: false },
    FormatEntry { token: "UINT8",  tier: Tier::T2NearRunnable, bits: 8,   exp_bits: None, mant_bits: None, storage: Storage::U8,   runnable: false },
    FormatEntry { token: "UINT16", tier: Tier::T9FixedEncoded, bits: 16,  exp_bits: None, mant_bits: None, storage: Storage::U16,  runnable: false },
    FormatEntry { token: "UINT32", tier: Tier::T9FixedEncoded, bits: 32,  exp_bits: None, mant_bits: None, storage: Storage::U32,  runnable: false },
    FormatEntry { token: "UINT64", tier: Tier::T9FixedEncoded, bits: 64,  exp_bits: None, mant_bits: None, storage: Storage::U64,  runnable: false },
    FormatEntry { token: "UINT128",tier: Tier::T4ExoticWide,   bits: 128, exp_bits: None, mant_bits: None, storage: Storage::U128, runnable: false },
    // ── Fixed / encoded (T9) ─────────────────────────────────────────────────
    FormatEntry { token: "Q-format", tier: Tier::T9FixedEncoded, bits: 32, exp_bits: None, mant_bits: None, storage: Storage::U32, runnable: false },
    FormatEntry { token: "BCD",      tier: Tier::T9FixedEncoded, bits: 8,  exp_bits: None, mant_bits: None, storage: Storage::U8,  runnable: false },
    FormatEntry { token: "minifloat",tier: Tier::T9FixedEncoded, bits: 8,  exp_bits: Some(4), mant_bits: Some(3), storage: Storage::U8, runnable: false },
    // ── Historical / archival (T7) ───────────────────────────────────────────
    FormatEntry { token: "IBM-HFP",    tier: Tier::T7Historical, bits: 32, exp_bits: Some(7), mant_bits: Some(24), storage: Storage::U32, runnable: false },
    FormatEntry { token: "MBF",        tier: Tier::T7Historical, bits: 32, exp_bits: Some(8), mant_bits: Some(24), storage: Storage::U32, runnable: false },
    FormatEntry { token: "VAX-F",      tier: Tier::T7Historical, bits: 32, exp_bits: Some(8), mant_bits: Some(23), storage: Storage::U32, runnable: false },
    FormatEntry { token: "VAX-D",      tier: Tier::T7Historical, bits: 64, exp_bits: Some(8), mant_bits: Some(55), storage: Storage::U64, runnable: false },
    FormatEntry { token: "VAX-G",      tier: Tier::T7Historical, bits: 64, exp_bits: Some(11), mant_bits: Some(52),storage: Storage::U64, runnable: false },
    FormatEntry { token: "VAX-H",      tier: Tier::T7Historical, bits: 128,exp_bits: Some(15), mant_bits: Some(112),storage: Storage::U128,runnable: false },
    FormatEntry { token: "Cray-float", tier: Tier::T7Historical, bits: 64, exp_bits: Some(15), mant_bits: Some(48), storage: Storage::U64, runnable: false },
    // ── Unum family (T8) ─────────────────────────────────────────────────────
    FormatEntry { token: "Unum-I",  tier: Tier::T8Unum, bits: 0, exp_bits: None, mant_bits: None, storage: Storage::Variable, runnable: false },
    FormatEntry { token: "Unum-II", tier: Tier::T8Unum, bits: 0, exp_bits: None, mant_bits: None, storage: Storage::Variable, runnable: false },
    // ── Tapered / block / stochastic (T3) ────────────────────────────────────
    FormatEntry { token: "tapered-fp",          tier: Tier::T3Research, bits: 0,  exp_bits: None, mant_bits: None, storage: Storage::Variable, runnable: false },
    FormatEntry { token: "block-fp",            tier: Tier::T3Research, bits: 0,  exp_bits: None, mant_bits: None, storage: Storage::Block,    runnable: false },
    FormatEntry { token: "shared-exponent",     tier: Tier::T3Research, bits: 0,  exp_bits: None, mant_bits: None, storage: Storage::Block,    runnable: false },
    FormatEntry { token: "stochastic-rounding", tier: Tier::T3Research, bits: 32, exp_bits: Some(8), mant_bits: Some(23), storage: Storage::U32, runnable: false },
];

/// Lookup by case-sensitive canonical token. Returns `None` if token is not catalog-listed.
pub fn lookup(token: &str) -> Option<&'static FormatEntry> {
    ALL.iter().find(|f| f.token == token)
}

/// All T1-runnable tokens (kernel ships in current GHCR image).
pub fn runnable() -> impl Iterator<Item = &'static FormatEntry> {
    ALL.iter().filter(|f| f.runnable)
}

/// Filter by tier.
pub fn by_tier(tier: Tier) -> impl Iterator<Item = &'static FormatEntry> {
    ALL.iter().filter(move |f| f.tier == tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_exactly_64_entries() {
        assert_eq!(
            ALL.len(),
            64,
            "operator-canonical count must be 64 (2026-05-02 instruction)"
        );
    }

    #[test]
    fn all_tokens_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for f in ALL {
            assert!(seen.insert(f.token), "duplicate token: {}", f.token);
        }
    }

    #[test]
    fn runnable_flag_matches_t1() {
        // Exactly four T1 formats today: binary16, binary32, bfloat16, GF16.
        let t1: Vec<_> = by_tier(Tier::T1Runnable).map(|f| f.token).collect();
        assert_eq!(t1, vec!["binary16", "binary32", "bfloat16", "GF16"]);
        let runnable_set: Vec<_> = runnable().map(|f| f.token).collect();
        assert_eq!(runnable_set, t1);
    }

    #[test]
    fn lookup_known_token() {
        assert_eq!(lookup("binary32").unwrap().bits, 32);
        assert_eq!(lookup("Posit32").unwrap().tier, Tier::T3Research);
        assert!(lookup("NOT_A_FORMAT").is_none());
    }

    #[test]
    fn all_tier_variants_covered() {
        use std::collections::BTreeSet;
        let tiers: BTreeSet<_> = ALL.iter().map(|f| f.tier.as_str()).collect();
        for expected in ["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8", "T9"] {
            assert!(tiers.contains(expected), "tier {} missing", expected);
        }
    }
}

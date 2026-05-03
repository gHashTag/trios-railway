//! `trios-numeric-catalog` — SSOT catalog of 63 numeric formats for IGLA fleet.
//!
//! Every format token used in experiment canon names (`IGLA-{LANE}-{format_token}-...`)
//! MUST resolve to exactly one [`NumericFormat`] variant.  No ad-hoc strings.
//!
//! # Tier model
//!
//! | Tier | Count | Status | Example |
//! |------|-------|--------|---------|
//! | T1   | 4     | runnable (kernel + CI pass) | binary32, binary16, bfloat16, gf16 |
//! | T2   | 10    | near-runnable (kernel WIP)  | tf32, fp8_e4m3, fp8_e5m2, … |
//! | T3   | 14    | spec-only (Posit/LNS/block-FP) | posit8, lns8, mxfp4, … |
//! | T4   | 6     | spec-only (128–256 bit)     | binary128, binary256, … |
//! | T5   | 9     | kernel WIP (sub-byte)       | fp6_e3m2, fp4_e2m1, gf4, … |
//! | T6   | 3     | spec-only (decimal)         | decimal32, decimal64, decimal128 |
//! | T7   | 7     | archival (historical)       | ibm_hfp, vax_f, cray_float, … |
//! | T8   | 2     | research (unum)             | unum1, unum2 |
//! | T9   | 8     | partial (fixed/encoded)     | q_format, bcd, minifloat, … |
//!
//! Total: 4+10+14+6+9+3+7+2+8 = **63**.

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

// ── Tier ────────────────────────────────────────────────────────────────────

/// Readiness tier for a numeric format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// Kernel exists, CI-green, safe to enqueue.
    T1,
    /// Kernel WIP, 2–3 days to runnable.
    T2,
    /// Spec-only (Posit/LNS/block-FP).
    T3,
    /// Exotic wide (128–256 bit).
    T4,
    /// Sub-byte (micro formats).
    T5,
    /// Decimal IEEE 754-2008.
    T6,
    /// Historical / archival (read-only).
    T7,
    /// Unum family (research).
    T8,
    /// Fixed-point / encoded.
    T9,
}

impl Tier {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::T1 => "runnable",
            Self::T2 => "near-runnable",
            Self::T3 => "spec-only",
            Self::T4 => "exotic-wide",
            Self::T5 => "sub-byte",
            Self::T6 => "decimal",
            Self::T7 => "historical",
            Self::T8 => "unum",
            Self::T9 => "fixed-encoded",
        }
    }

    /// Can experiments be enqueued for this tier?
    pub fn runnable(&self) -> bool {
        matches!(self, Self::T1)
    }
}

// ── Storage type ────────────────────────────────────────────────────────────

/// Underlying storage width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    U8,
    U16,
    U32,
    U64,
    U128,
    /// Variable-length (Posit, Unum).
    Variable,
}

// ── NumericFormat (63 variants) ─────────────────────────────────────────────

/// Canonical numeric format — all 63 tokens.
///
/// Anchor: `φ² + φ⁻² = 3`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NumericFormat {
    // ── T1: runnable (4) ────────────────────────────────────────────────
    /// IEEE 754 binary32 (single precision).
    Binary32,
    /// IEEE 754 binary16 (half precision).
    Binary16,
    /// Google Brain float16.
    Bfloat16,
    /// Golden Float 16 (φ-anchored).
    Gf16,

    // ── T2: near-runnable (10) ──────────────────────────────────────────
    /// IEEE 754 binary64 (double precision).
    Binary64,
    /// NVIDIA TensorFloat-32.
    Tf32,
    /// FP8 with E4M3 bias (NVIDIA/AMD).
    Fp8E4m3,
    /// FP8 with E5M2 bias (NVIDIA/AMD).
    Fp8E5m2,
    /// Golden Float 8.
    Gf8,
    /// Golden Float 32.
    Gf32,
    /// Integer 8-bit.
    Int8,
    /// Integer 16-bit.
    Int16,
    /// Integer 32-bit.
    Int32,
    /// Unsigned integer 8-bit.
    Uint8,

    // ── T3: research — Posit/LNS/block-FP (14) ─────────────────────────
    /// Posit<8,0>.
    Posit8,
    /// Posit<16,1>.
    Posit16,
    /// Posit<32,2>.
    Posit32,
    /// Posit<64,3>.
    Posit64,
    /// Logarithmic Number System (8-bit).
    Lns8,
    /// Microscaling FP8.
    Mxfp8,
    /// Microscaling FP6.
    Mxfp6,
    /// Microscaling FP4.
    Mxfp4,
    /// NormalFloat-4 (QLoRA).
    Nf4,
    /// Alternating Float Point.
    Afp,
    /// Block Floating Point.
    BlockFp,
    /// Shared Exponent (bfloat-like).
    SharedExponent,
    /// Stochastic Rounding wrapper.
    StochasticRounding,
    /// Tapered Floating Point.
    TaperedFp,

    // ── T4: exotic wide (6) ─────────────────────────────────────────────
    /// IEEE 754 binary128 (quad precision).
    Binary128,
    /// IEEE 754 binary256 (oct precision).
    Binary256,
    /// Double-Double arithmetic.
    DoubleDouble,
    /// Quad-Double arithmetic.
    QuadDouble,
    /// Intel extended precision 80-bit.
    Fp80,
    /// Integer 128-bit.
    Int128,

    // ── T5: sub-byte / micro (9) ────────────────────────────────────────
    /// FP6 E3M2.
    Fp6E3m2,
    /// FP6 E2M3.
    Fp6E2m3,
    /// FP4 E2M1.
    Fp4E2m1,
    /// Golden Float 4.
    Gf4,
    /// Golden Float 12.
    Gf12,
    /// Golden Float 20.
    Gf20,
    /// Golden Float 24.
    Gf24,
    /// Integer 4-bit.
    Int4,
    /// Unsigned integer 4-bit.
    Uint4,

    // ── T6: decimal (3) ─────────────────────────────────────────────────
    /// IEEE 754-2008 decimal32.
    Decimal32,
    /// IEEE 754-2008 decimal64.
    Decimal64,
    /// IEEE 754-2008 decimal128.
    Decimal128,

    // ── T7: historical / archival (7) ───────────────────────────────────
    /// IBM Hexadecimal Floating Point.
    IbmHfp,
    /// Microsoft Binary Format.
    Mbf,
    /// VAX F_floating.
    VaxF,
    /// VAX D_floating.
    VaxD,
    /// VAX G_floating.
    VaxG,
    /// VAX H_floating.
    VaxH,
    /// Cray floating point.
    CrayFloat,

    // ── T8: unum family (2) ─────────────────────────────────────────────
    /// Unum Type I.
    Unum1,
    /// Unum Type II (Posit + valids).
    Unum2,

    // ── T9: fixed / encoded (8) ─────────────────────────────────────────
    /// Q-format fixed point.
    QFormat,
    /// Binary Coded Decimal.
    Bcd,
    /// Minifloat (custom exponent/mantissa).
    Minifloat,
    /// Integer 64-bit.
    Int64,
    /// Unsigned integer 64-bit.
    Uint64,
    /// Unsigned integer 128-bit.
    Uint128,
    /// Floating Point with overflow saturation.
    SaturatedFp,
    /// Fixed-point accumulator.
    FixedAccum,
}

impl NumericFormat {
    /// Canonical token for experiment names (e.g. `"gf16"`, `"fp8_e4m3"`).
    pub fn token(&self) -> &'static str {
        self.into()
    }

    /// Readiness tier.
    pub fn tier(&self) -> Tier {
        match self {
            // T1
            Self::Binary32 | Self::Binary16 | Self::Bfloat16 | Self::Gf16 => Tier::T1,
            // T2
            Self::Binary64
            | Self::Tf32
            | Self::Fp8E4m3
            | Self::Fp8E5m2
            | Self::Gf8
            | Self::Gf32
            | Self::Int8
            | Self::Int16
            | Self::Int32
            | Self::Uint8 => Tier::T2,
            // T3
            Self::Posit8
            | Self::Posit16
            | Self::Posit32
            | Self::Posit64
            | Self::Lns8
            | Self::Mxfp8
            | Self::Mxfp6
            | Self::Mxfp4
            | Self::Nf4
            | Self::Afp
            | Self::BlockFp
            | Self::SharedExponent
            | Self::StochasticRounding
            | Self::TaperedFp => Tier::T3,
            // T4
            Self::Binary128
            | Self::Binary256
            | Self::DoubleDouble
            | Self::QuadDouble
            | Self::Fp80
            | Self::Int128 => Tier::T4,
            // T5
            Self::Fp6E3m2
            | Self::Fp6E2m3
            | Self::Fp4E2m1
            | Self::Gf4
            | Self::Gf12
            | Self::Gf20
            | Self::Gf24
            | Self::Int4
            | Self::Uint4 => Tier::T5,
            // T6
            Self::Decimal32 | Self::Decimal64 | Self::Decimal128 => Tier::T6,
            // T7
            Self::IbmHfp
            | Self::Mbf
            | Self::VaxF
            | Self::VaxD
            | Self::VaxG
            | Self::VaxH
            | Self::CrayFloat => Tier::T7,
            // T8
            Self::Unum1 | Self::Unum2 => Tier::T8,
            // T9
            Self::QFormat
            | Self::Bcd
            | Self::Minifloat
            | Self::Int64
            | Self::Uint64
            | Self::Uint128
            | Self::SaturatedFp
            | Self::FixedAccum => Tier::T9,
        }
    }

    /// Can this format be used in experiments right now?
    pub fn runnable(&self) -> bool {
        self.tier().runnable()
    }

    /// Total bit-width of the format.
    pub fn bits(&self) -> u32 {
        match self {
            Self::Binary32 | Self::Tf32 => 32,
            Self::Binary16 | Self::Bfloat16 | Self::Gf16 => 16,
            Self::Binary64 | Self::Gf32 => 64,
            Self::Fp8E4m3 | Self::Fp8E5m2 | Self::Gf8 | Self::Int8 | Self::Uint8 => 8,
            Self::Int16 => 16,
            Self::Int32 => 32,
            Self::Posit8 => 8,
            Self::Posit16 => 16,
            Self::Posit32 => 32,
            Self::Posit64 => 64,
            Self::Lns8 => 8,
            Self::Mxfp8 => 8,
            Self::Mxfp6 => 6,
            Self::Mxfp4 => 4,
            Self::Nf4 => 4,
            Self::Afp => 8,
            Self::BlockFp => 8,
            Self::SharedExponent => 8,
            Self::StochasticRounding => 8,
            Self::TaperedFp => 8,
            Self::Binary128 => 128,
            Self::Binary256 => 256,
            Self::DoubleDouble => 128,
            Self::QuadDouble => 256,
            Self::Fp80 => 80,
            Self::Int128 | Self::Uint128 => 128,
            Self::Fp6E3m2 | Self::Fp6E2m3 => 6,
            Self::Fp4E2m1 | Self::Gf4 | Self::Int4 | Self::Uint4 => 4,
            Self::Gf12 => 12,
            Self::Gf20 => 20,
            Self::Gf24 => 24,
            Self::Decimal32 => 32,
            Self::Decimal64 => 64,
            Self::Decimal128 => 128,
            Self::IbmHfp | Self::Mbf => 32,
            Self::VaxF => 32,
            Self::VaxD => 64,
            Self::VaxG => 64,
            Self::VaxH => 128,
            Self::CrayFloat => 64,
            Self::Unum1 | Self::Unum2 => 0, // variable
            Self::QFormat => 16,
            Self::Bcd => 8,
            Self::Minifloat => 8,
            Self::Int64 | Self::Uint64 => 64,
            Self::SaturatedFp => 8,
            Self::FixedAccum => 32,
        }
    }

    /// Exponent bits, if applicable.
    pub fn exp_bits(&self) -> Option<u32> {
        match self {
            Self::Binary32 => Some(8),
            Self::Binary16 => Some(5),
            Self::Bfloat16 => Some(8),
            Self::Gf16 => Some(5),
            Self::Binary64 => Some(11),
            Self::Tf32 => Some(8),
            Self::Fp8E4m3 => Some(4),
            Self::Fp8E5m2 => Some(5),
            Self::Gf8 => Some(4),
            Self::Gf32 => Some(8),
            Self::Fp6E3m2 => Some(3),
            Self::Fp6E2m3 => Some(2),
            Self::Fp4E2m1 => Some(2),
            Self::Gf4 => Some(2),
            Self::Gf12 => Some(4),
            Self::Gf20 => Some(5),
            Self::Gf24 => Some(6),
            Self::Binary128 => Some(15),
            Self::Binary256 => Some(19),
            Self::Fp80 => Some(15),
            Self::Decimal32 => Some(8),
            Self::Decimal64 => Some(10),
            Self::Decimal128 => Some(14),
            Self::Minifloat => Some(4),
            _ => None,
        }
    }

    /// Mantissa bits, if applicable.
    pub fn mant_bits(&self) -> Option<u32> {
        match self {
            Self::Binary32 => Some(23),
            Self::Binary16 => Some(10),
            Self::Bfloat16 => Some(7),
            Self::Gf16 => Some(10),
            Self::Binary64 => Some(52),
            Self::Tf32 => Some(10),
            Self::Fp8E4m3 => Some(3),
            Self::Fp8E5m2 => Some(2),
            Self::Gf8 => Some(3),
            Self::Gf32 => Some(23),
            Self::Fp6E3m2 => Some(2),
            Self::Fp6E2m3 => Some(3),
            Self::Fp4E2m1 => Some(1),
            Self::Gf4 => Some(1),
            Self::Gf12 => Some(7),
            Self::Gf20 => Some(14),
            Self::Gf24 => Some(17),
            Self::Binary128 => Some(112),
            Self::Binary256 => Some(236),
            Self::Fp80 => Some(64),
            Self::Minifloat => Some(3),
            _ => None,
        }
    }

    /// Storage type for the format.
    pub fn storage_type(&self) -> StorageType {
        match self.bits() {
            0 => StorageType::Variable,
            1..=8 => StorageType::U8,
            9..=16 => StorageType::U16,
            17..=32 => StorageType::U32,
            33..=64 => StorageType::U64,
            _ => StorageType::U128,
        }
    }

    /// φ-distance from GF16 (the golden reference format).
    ///
    /// GF16 has distance 0.  Formats closer to GF16 in bit-structure
    /// have smaller φ-distance.  `None` for non-floating formats.
    pub fn phi_distance(&self) -> Option<f64> {
        match self {
            Self::Gf16 => Some(0.0),
            Self::Binary16 => Some(0.618033988749895),   // φ⁻¹
            Self::Bfloat16 => Some(1.0),                  // same bits, different split
            Self::Fp8E4m3 => Some(1.23606797749979),      // φ⁻¹ + φ⁻²
            Self::Fp8E5m2 => Some(1.38196601125011),      // φ⁻¹ + φ⁻² + δ
            Self::Gf8 => Some(0.38196601125011),          // φ⁻²
            Self::Binary32 => Some(1.618033988749895),    // φ
            Self::Tf32 => Some(1.0),                      // same as bf16 distance
            Self::Gf32 => Some(0.23606797749979),         // φ⁻³
            Self::Binary64 => Some(2.618033988749895),    // φ²
            Self::Gf4 => Some(0.14589803375031),          // φ⁻⁴
            Self::Gf12 => Some(0.09016994374947),         // φ⁻⁵
            Self::Gf20 => Some(0.05572809000084),         // φ⁻⁶
            Self::Gf24 => Some(0.03444185374863),         // φ⁻⁷
            _ => None,
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Binary32 => "IEEE 754 single precision (32-bit)",
            Self::Binary16 => "IEEE 754 half precision (16-bit)",
            Self::Bfloat16 => "Google Brain float16 (16-bit, 8-exp)",
            Self::Gf16 => "Golden Float 16 (φ-anchored, 16-bit)",
            Self::Binary64 => "IEEE 754 double precision (64-bit)",
            Self::Tf32 => "NVIDIA TensorFloat-32 (19-bit: 8-exp, 10-mant)",
            Self::Fp8E4m3 => "FP8 E4M3 (NVIDIA/AMD, 8-bit)",
            Self::Fp8E5m2 => "FP8 E5M2 (NVIDIA/AMD, 8-bit)",
            Self::Gf8 => "Golden Float 8 (φ-anchored, 8-bit)",
            Self::Gf32 => "Golden Float 32 (φ-anchored, 32-bit)",
            Self::Int8 => "Signed integer 8-bit",
            Self::Int16 => "Signed integer 16-bit",
            Self::Int32 => "Signed integer 32-bit",
            Self::Uint8 => "Unsigned integer 8-bit",
            Self::Posit8 => "Posit<8,0> (Type III unum)",
            Self::Posit16 => "Posit<16,1> (Type III unum)",
            Self::Posit32 => "Posit<32,2> (Type III unum)",
            Self::Posit64 => "Posit<64,3> (Type III unum)",
            Self::Lns8 => "Logarithmic Number System (8-bit)",
            Self::Mxfp8 => "OCP Microscaling FP8",
            Self::Mxfp6 => "OCP Microscaling FP6",
            Self::Mxfp4 => "OCP Microscaling FP4",
            Self::Nf4 => "NormalFloat-4 (QLoRA quantization)",
            Self::Afp => "Alternating Float Point",
            Self::BlockFp => "Block Floating Point",
            Self::SharedExponent => "Shared Exponent (bfloat-like)",
            Self::StochasticRounding => "Stochastic Rounding wrapper",
            Self::TaperedFp => "Tapered Floating Point",
            Self::Binary128 => "IEEE 754 quad precision (128-bit)",
            Self::Binary256 => "IEEE 754 oct precision (256-bit)",
            Self::DoubleDouble => "Double-Double arithmetic (128-bit effective)",
            Self::QuadDouble => "Quad-Double arithmetic (256-bit effective)",
            Self::Fp80 => "Intel extended precision (80-bit)",
            Self::Int128 => "Signed integer 128-bit",
            Self::Fp6E3m2 => "FP6 E3M2 (6-bit float)",
            Self::Fp6E2m3 => "FP6 E2M3 (6-bit float)",
            Self::Fp4E2m1 => "FP4 E2M1 (4-bit float)",
            Self::Gf4 => "Golden Float 4 (φ-anchored, 4-bit)",
            Self::Gf12 => "Golden Float 12 (φ-anchored, 12-bit)",
            Self::Gf20 => "Golden Float 20 (φ-anchored, 20-bit)",
            Self::Gf24 => "Golden Float 24 (φ-anchored, 24-bit)",
            Self::Int4 => "Signed integer 4-bit",
            Self::Uint4 => "Unsigned integer 4-bit",
            Self::Decimal32 => "IEEE 754-2008 decimal32",
            Self::Decimal64 => "IEEE 754-2008 decimal64",
            Self::Decimal128 => "IEEE 754-2008 decimal128",
            Self::IbmHfp => "IBM Hexadecimal Floating Point (32-bit)",
            Self::Mbf => "Microsoft Binary Format (32-bit)",
            Self::VaxF => "VAX F_floating (32-bit)",
            Self::VaxD => "VAX D_floating (64-bit)",
            Self::VaxG => "VAX G_floating (64-bit)",
            Self::VaxH => "VAX H_floating (128-bit)",
            Self::CrayFloat => "Cray floating point (64-bit)",
            Self::Unum1 => "Unum Type I (variable width)",
            Self::Unum2 => "Unum Type II (Posit + valids)",
            Self::QFormat => "Q-format fixed point (16-bit)",
            Self::Bcd => "Binary Coded Decimal (8-bit)",
            Self::Minifloat => "Minifloat (custom 8-bit)",
            Self::Int64 => "Signed integer 64-bit",
            Self::Uint64 => "Unsigned integer 64-bit",
            Self::Uint128 => "Unsigned integer 128-bit",
            Self::SaturatedFp => "Saturated floating point (8-bit)",
            Self::FixedAccum => "Fixed-point accumulator (32-bit)",
        }
    }

    /// All 63 formats in tier order.
    pub fn all() -> &'static [NumericFormat; 63] {
        use NumericFormat::*;
        &[
            // T1 (4)
            Binary32, Binary16, Bfloat16, Gf16,
            // T2 (10)
            Binary64, Tf32, Fp8E4m3, Fp8E5m2, Gf8, Gf32, Int8, Int16, Int32, Uint8,
            // T3 (14)
            Posit8, Posit16, Posit32, Posit64, Lns8, Mxfp8, Mxfp6, Mxfp4, Nf4, Afp, BlockFp,
            SharedExponent, StochasticRounding, TaperedFp,
            // T4 (6)
            Binary128, Binary256, DoubleDouble, QuadDouble, Fp80, Int128,
            // T5 (9)
            Fp6E3m2, Fp6E2m3, Fp4E2m1, Gf4, Gf12, Gf20, Gf24, Int4, Uint4,
            // T6 (3)
            Decimal32, Decimal64, Decimal128,
            // T7 (7)
            IbmHfp, Mbf, VaxF, VaxD, VaxG, VaxH, CrayFloat,
            // T8 (2)
            Unum1, Unum2,
            // T9 (8)
            QFormat, Bcd, Minifloat, Int64, Uint64, Uint128, SaturatedFp, FixedAccum,
        ]
    }

    /// Formats that can run experiments right now (T1).
    pub fn runnable_formats() -> &'static [NumericFormat; 4] {
        use NumericFormat::*;
        &[Binary32, Binary16, Bfloat16, Gf16]
    }

    /// T1 + T2 formats (14 total, Plan B).
    pub fn plan_b_formats() -> &'static [NumericFormat; 14] {
        use NumericFormat::*;
        &[
            Binary32, Binary16, Bfloat16, Gf16,
            Binary64, Tf32, Fp8E4m3, Fp8E5m2, Gf8, Gf32, Int8, Int16, Int32, Uint8,
        ]
    }

    /// Parse a token string to a format (case-insensitive).
    pub fn from_token(token: &str) -> Option<Self> {
        Self::all().iter().find(|f| f.token().eq_ignore_ascii_case(token)).copied()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_is_exactly_63() {
        assert_eq!(NumericFormat::all().len(), 63, "catalog must have exactly 63 formats");
    }

    #[test]
    fn all_tokens_unique() {
        let mut tokens: Vec<&str> = NumericFormat::all().iter().map(|f| f.token()).collect();
        tokens.sort();
        let dedup = tokens.clone();
        tokens.dedup();
        assert_eq!(tokens.len(), dedup.len(), "duplicate tokens found");
    }

    #[test]
    fn t1_is_runnable() {
        for f in NumericFormat::runnable_formats() {
            assert!(f.runnable(), "{:?} should be runnable", f);
            assert_eq!(f.tier(), Tier::T1, "{:?} should be T1", f);
        }
    }

    #[test]
    fn t2_is_not_runnable_yet() {
        use NumericFormat::*;
        for f in [Binary64, Tf32, Fp8E4m3, Fp8E5m2, Gf8, Gf32, Int8, Int16, Int32, Uint8] {
            assert!(!f.runnable(), "{:?} should NOT be runnable", f);
            assert_eq!(f.tier(), Tier::T2, "{:?} should be T2", f);
        }
    }

    #[test]
    fn plan_b_has_14_formats() {
        assert_eq!(NumericFormat::plan_b_formats().len(), 14);
    }

    #[test]
    fn from_token_roundtrip() {
        for f in NumericFormat::all() {
            let token = f.token();
            assert_eq!(NumericFormat::from_token(token), Some(*f));
        }
    }

    #[test]
    fn from_token_case_insensitive() {
        assert_eq!(NumericFormat::from_token("GF16"), Some(NumericFormat::Gf16));
        assert_eq!(NumericFormat::from_token("BINARY32"), Some(NumericFormat::Binary32));
        assert_eq!(NumericFormat::from_token("fp8_e4m3"), Some(NumericFormat::Fp8E4m3));
    }

    #[test]
    fn gf16_phi_distance_is_zero() {
        assert_eq!(NumericFormat::Gf16.phi_distance(), Some(0.0));
    }

    #[test]
    fn tier_labels_exist() {
        for f in NumericFormat::all() {
            let _label = f.tier().label();
        }
    }

    #[test]
    fn bits_are_positive_except_variable() {
        for f in NumericFormat::all() {
            if matches!(f, NumericFormat::Unum1 | NumericFormat::Unum2) {
                assert_eq!(f.bits(), 0);
                assert_eq!(f.storage_type(), StorageType::Variable);
            } else {
                assert!(f.bits() > 0, "{:?} should have positive bits", f);
            }
        }
    }
}

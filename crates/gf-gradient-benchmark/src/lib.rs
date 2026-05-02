//! gf-gradient-benchmark — Gradient stability benchmark for φ-quantized formats
//!
//! Evaluates 12 precision formats (GF4, GF4a, GF6a, GF8, GF12, GF16,
//! GF20, GF24, GF32, GF64, FP16, BF16, FP32) across 4 loss surfaces
//! with 5 seeds each = 240 reproducible experiments.
//!
//! ## Output Format
//!
//! Results are written to `results/gradient_metrics_{format}_{surface}_{seed}.csv`
//! with columns:
//! - `step`: Optimization step number
//! - `loss`: L(θ) at this step
//! - `grad_norm`: ‖∇L‖₂ — gradient stability measure
//! - `grad_var`: σ²(∇) — gradient noise measure
//! - `snr`: Signal-to-noise ratio(∇)
//! - `bias_vs_fp32`: ‖∇_format − ∇_fp32‖₂ — systematic error
//!
//! ## References
//!
//! - zig-golden-float#12 (BENCH-007): φ-optimized format specifications
//! - trios-trainer-igla#50: IGLA evaluation semantics
//! - trios#331: Target BPB < 1.50 (compression goal)

pub mod surfaces;
pub mod quantize;
pub mod metrics;
pub mod runner;

pub use surfaces::LossSurface;
pub use quantize::FormatQuantizer;
pub use metrics::{
    compute_grad_norm,
    compute_grad_variance,
    compute_snr,
    compute_bias_vs_fp32,
};

/// All 12 precision formats to benchmark
pub const ALL_FORMATS: [&str] = &[
    "GF4",
    "GF4a",
    "GF6a",
    "GF8",
    "GF12",
    "GF16",
    "GF20",
    "GF24",
    "GF32",
    "GF64",
    "FP16",
    "BF16",
    "FP32",
];

/// Seeds for reproducible experiments
pub const SEEDS: [u64] = &[42, 43, 44, 45, 46];

/// Number of steps per benchmark run
pub const STEPS_PER_RUN: usize = 1000;

/// Learning rate η = α × φ⁻³ (adaptive)
/// φ⁻³ ≈ 0.23606797749979
/// α is typically in [0.01, 0.1]
pub fn adaptive_learning_rate(alpha: f64, step: usize) -> f64 {
    // Could implement learning rate schedule here
    alpha * 0.23606797749979
}

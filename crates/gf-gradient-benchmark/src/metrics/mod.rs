//! Gradient metrics for quantization analysis
//!
//! Computes:
//! - Gradient norm: ‖∇L‖₂ — stability indicator
//! - Gradient variance: σ²(∇) — noise measure
//! - SNR: Signal-to-noise ratio(∇) — quality metric
//! - Bias vs FP32: ‖∇_format − ∇_fp32‖₂ — systematic error

use nalgebra::DVector;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Compute Euclidean norm (L2 norm) of gradient
///
/// ‖∇L‖₂ = √(Σ ∇L(θ_i)²)
pub fn compute_grad_norm(grad: &[f64]) -> f64 {
    grad.iter().map(|&x| x * x).sum::<f64>().sqrt()
}

/// Compute variance of gradient
///
/// σ²(∇) = Var(∇L(θ_i))
/// Computes variance across parameter dimensions
pub fn compute_grad_variance(grad: &[f64]) -> f64 {
    let n = grad.len() as f64;
    if n <= 1.0 {
        return 0.0;
    }

    let mean: f64 = grad.iter().sum::<f64>() / n;
    grad.iter().map(|&x| {
        let diff = x - mean;
        diff * diff
    }).sum::<f64>() / n
}

/// Compute signal-to-noise ratio
///
/// SNR(∇) = ‖∇‖₂ / ‖∇ - FP32(∇)‖₂
/// Higher SNR means format gradient is closer to FP32 gradient
pub fn compute_snr(format_grad: &[f64], fp32_grad: &[f64]) -> f64 {
    let format_norm = compute_grad_norm(format_grad);
    let diff_norm = format_grad
        .iter()
        .zip(fp32_grad)
        .map(|(a, b)| a - b)
        .collect::<Vec<_>>();

    let diff_norm = compute_grad_norm(&diff_norm);

    if diff_norm < f64::EPSILON {
        return f64::INFINITY;
    }

    format_norm / diff_norm
}

/// Compute bias vs FP32
///
/// ‖∇_format − ∇_fp32‖₂ — systematic error
pub fn compute_bias_vs_fp32(format_grad: &[f64], fp32_grad: &[f64]) -> f64 {
    let diff: Vec<f64> = format_grad
        .iter()
        .zip(fp32_grad)
        .map(|(a, b)| a - b)
        .collect();

    compute_grad_norm(&diff)
}

/// Track convergence trajectory
///
/// Records loss and gradient history for analysis
pub struct ConvergenceTracker {
    pub format_name: String,
    pub surface_name: String,
    pub seed: u64,
    steps: Vec<StepMetrics>,
}

impl ConvergenceTracker {
    pub fn new(format_name: &str, surface_name: &str, seed: u64) -> Self {
        Self {
            format_name: format_name.to_string(),
            surface_name: surface_name.to_string(),
            seed,
            steps: Vec::with_capacity(1001),
        }
    }

    /// Record metrics for a step
    pub fn record(&mut self, step: usize, loss: f64, grad_norm: f64,
                 grad_var: f64, snr: f64, bias_vs_fp32: f64) {
        self.steps.push(StepMetrics {
            step,
            loss,
            grad_norm,
            grad_var,
            snr,
            bias_vs_fp32,
        });
    }

    /// Get final metrics
    pub fn final_metrics(&self) -> Option<&StepMetrics> {
        self.steps.last()
    }
}

#[derive(Clone, Debug)]
pub struct StepMetrics {
    pub step: usize,
    pub loss: f64,
    pub grad_norm: f64,
    pub grad_var: f64,
    pub snr: f64,
    pub bias_vs_fp32: f64,
}

/// Batch compute metrics for multiple format gradients
///
/// Compares all format gradients against FP32 baseline
pub struct BatchMetrics {
    pub fp32_grad: Vec<f64>,
}

impl BatchMetrics {
    pub fn new(fp32_grad: Vec<f64>) -> Self {
        Self { fp32_grad }
    }

    /// Compute metrics for a single format gradient
    pub fn compute_for_format(&self, format_grad: &[f64]) -> FormatMetrics {
        FormatMetrics {
            grad_norm: compute_grad_norm(format_grad),
            grad_variance: compute_grad_variance(format_grad),
            snr: compute_snr(format_grad, &self.fp32_grad),
            bias_vs_fp32: compute_bias_vs_fp32(format_grad, &self.fp32_grad),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FormatMetrics {
    pub grad_norm: f64,
    pub grad_variance: f64,
    pub snr: f64,
    pub bias_vs_fp32: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grad_norm() {
        let grad = vec![3.0, 4.0, 0.0];
        let norm = compute_grad_norm(&grad);
        assert!((norm - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_grad_variance() {
        let grad = vec![2.0, 4.0, 6.0];
        let var = compute_grad_variance(&grad);
        assert!((var - 2.66666).abs() < f64::EPSILON);
    }

    #[test]
    fn test_snr() {
        let fp32 = vec![10.0, 10.0, 10.0];
        let format = vec![9.0, 11.0, 10.0];
        let snr = compute_snr(&format, &fp32);
        assert!(snr > 1.0); // format worse than FP32
    }

    #[test]
    fn test_bias_vs_fp32() {
        let fp32 = vec![1.0, 2.0, 3.0];
        let format = vec![1.1, 1.9, 3.1];
        let bias = compute_bias_vs_fp32(&format, &fp32);
        assert!((bias - 0.033333).abs() < f64::EPSILON);
    }

    #[test]
    fn test_convergence_tracker() {
        let mut tracker = ConvergenceTracker::new("GF16", "Rosenbrock", 42);
        tracker.record(0, 100.0, 5.0, 0.25, 4.0, 2.5);
        tracker.record(1, 50.0, 2.5, 0.16, 3.0, 1.5);

        let final = tracker.final_metrics();
        assert_eq!(final.unwrap().step, 1);
        assert_eq!(final.unwrap().loss, 50.0);
    }
}

//! Loss surfaces for gradient benchmarking
//!
//! Provides synthetic loss functions with analytically computable gradients
//! to test gradient stability under quantization.

use nalgebra::{DVector, DynVector};
use rand::{Rng, SeedableRng, seq::SliceRandom};

/// Trait defining a loss surface
pub trait LossSurface: Send + Sync {
    /// Name of this surface
    fn name(&self) -> &'static str;

    /// Dimensionality of parameter space
    fn dimensions(&self) -> usize;

    /// Sample initial parameters θ₀
    fn sample_params<R: Rng>(&self, rng: &mut R) -> Vec<f64>;

    /// Compute loss L(θ)
    fn compute_loss(&self, params: &[f64]) -> f64;

    /// Compute exact gradient ∇L(θ)
    fn compute_gradient(&self, params: &[f64]) -> Vec<f64>;
}

// ============================================================================
// Rosenbrock Function (non-convex test function)
// ============================================================================

/// Rosenbrock: classic non-convex test function
///
/// L(θ) = Σ[i=0 to n-1] [100(θ_{i+1} - θ_i²)² + (1 - θ_i)²]
/// Global minimum at θ_i = 1 for all i
pub struct Rosenbrock {
    pub dim: usize,
}

impl Rosenbrock {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl LossSurface for Rosenbrock {
    fn name(&self) -> &'static str {
        "Rosenbrock"
    }

    fn dimensions(&self) -> usize {
        self.dim
    }

    fn sample_params<R: Rng>(&self, rng: &mut R) -> Vec<f64> {
        // Sample in range [-5, 5]
        (0..self.dim)
            .map(|_| rng.gen_range(-5.0_f64..5.0_f64))
            .collect()
    }

    fn compute_loss(&self, params: &[f64]) -> f64 {
        let n = params.len();
        let mut loss = 0.0_f64;

        for i in 0..n.saturating_sub(1) {
            let x = params[i];
            let next = params[i + 1];
            let term = 100.0 * (next - x * x).powi(2);
            let term2 = (1.0 - x).powi(2);
            loss += term + term2;
        }

        loss
    }

    fn compute_gradient(&self, params: &[f64]) -> Vec<f64> {
        let n = params.len();
        let mut grad = vec![0.0_f64; n];

        for i in 0..n.saturating_sub(1) {
            let x = params[i];
            let next = params[i + 1];

            // d/dx_i term
            let term1 = 100.0 * 2.0 * (next - x * x) * (-1.0);
            let term2 = 2.0 * (1.0 - x) * (-1.0);

            grad[i] += term1 + term2;
        }

        // d/dx_{i+1} term
        if n > 1 {
            let x = params[n - 2];
            let x_next = params[n - 1];
            let x_current = params[n - 1];

            let term = 100.0 * 2.0 * (x_current - x * x) * 1.0;
            let term2 = 2.0 * (1.0 - x_current) * 1.0;

            grad[n - 1] += term + term2;
        }

        grad
    }
}

// ============================================================================
// Quadratic Bowl (convex baseline)
// ============================================================================

/// Quadratic: simple convex bowl L(θ) = Σ θ_i²
///
/// Global minimum at θ_i = 0 for all i
pub struct Quadratic {
    pub dim: usize,
}

impl Quadratic {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl LossSurface for Quadratic {
    fn name(&self) -> &'static str {
        "Quadratic"
    }

    fn dimensions(&self) -> usize {
        self.dim
    }

    fn sample_params<R: Rng>(&self, rng: &mut R) -> Vec<f64> {
        // Sample in range [-5, 5]
        (0..self.dim)
            .map(|_| rng.gen_range(-5.0_f64..5.0_f64))
            .collect()
    }

    fn compute_loss(&self, params: &[f64]) -> f64 {
        params.iter().map(|&x| x * x).sum()
    }

    fn compute_gradient(&self, params: &[f64]) -> Vec<f64> {
        params.iter().map(|&x| 2.0 * x).collect()
    }
}

// ============================================================================
// Rastrigin (multi-modal stress test)
// ============================================================================

/// Rastrigin: multi-modal with many local minima
///
/// L(θ) = 10n + Σ[i=1 to n] [θ_i² - 10 cos(2πθ_i)]
/// Global minimum at θ_i = 0, L(θ) = 0
pub struct Rastrigin {
    pub dim: usize,
}

impl Rastrigin {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl LossSurface for Rastrigin {
    fn name(&self) -> &'static str {
        "Rastrigin"
    }

    fn dimensions(&self) -> usize {
        self.dim
    }

    fn sample_params<R: Rng>(&self, rng: &mut R) -> Vec<f64> {
        // Sample in range [-5.12, 5.12] (standard range)
        (0..self.dim)
            .map(|_| rng.gen_range(-5.12_f64..5.12_f64))
            .collect()
    }

    fn compute_loss(&self, params: &[f64]) -> f64 {
        let n = params.len() as f64;
        let sum_sq: f64 = params.iter().map(|&x| x * x).sum();
        let sum_cos: f64 = params
            .iter()
            .enumerate()
            .map(|(i, &x)| 10.0 * (2.0 * std::f64::consts::PI * (i as f64 + 1.0) * x).cos())
            .sum();

        10.0 * n + sum_sq - sum_cos
    }

    fn compute_gradient(&self, params: &[f64]) -> Vec<f64> {
        params
            .iter()
            .enumerate()
            .map(|(i, &x)| 2.0 * x + 20.0 * std::f64::consts::PI * (i as f64 + 1.0) * x.sin())
            .collect()
    }
}

// ============================================================================
// MNIST Proxy (simple classifier simulation)
// ============================================================================

/// MNIST Proxy: simulates a simple classifier loss
///
/// L(θ, X, y) = CrossEntropy(Softmax(θ @ X), y)
/// Simplified: L(θ) = -log(softmax(θ₀)) for single class
pub struct MNISTProxy {
    pub num_classes: usize,
}

impl MNISTProxy {
    pub fn new(num_classes: usize) -> Self {
        Self { num_classes }
    }
}

impl LossSurface for MNISTProxy {
    fn name(&self) -> &'static str {
        "MNISTProxy"
    }

    fn dimensions(&self) -> usize {
        self.num_classes
    }

    fn sample_params<R: Rng>(&self, rng: &mut R) -> Vec<f64> {
        // Sample as log probabilities (initialized near uniform)
        let n = self.num_classes;
        let mut params = Vec::with_capacity(n);

        // Initialize as uniform log-probs
        let uniform = (-1.0 / n as f64).ln();
        for _ in 0..n {
            params.push(uniform + rng.gen_range(-0.1_f64..0.1_f64));
        }

        params
    }

    fn compute_loss(&self, params: &[f64]) -> f64 {
        // Simulate classification: use first parameter as "correct" class
        // L = -log(softmax(θ)[target_class])
        let log_sum_exp: f64 = log_sum_exp(params);
        let target_class = 0; // Assume first class is target
        let target_logit = params[target_class];

        -(target_logit - log_sum_exp)
    }

    fn compute_gradient(&self, params: &[f64]) -> Vec<f64> {
        let n = params.len();
        let log_sum_exp = log_sum_exp(params);
        let target_class = 0;

        let mut grad = vec![0.0_f64; n];

        // Gradient for target class
        grad[target_class] = 1.0 - (params[target_class] / log_sum_exp.exp());

        // Gradients for other classes
        for i in 0..n {
            if i != target_class {
                grad[i] = 0.0 - (params[i] / log_sum_exp.exp());
            }
        }

        grad
    }
}

/// Log-sum-exp for numerical stability
fn log_sum_exp(xs: &[f64]) -> f64 {
    let max_x = *xs.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let sum: f64 = xs.iter().map(|&x| (x - max_x).exp()).sum();
    max_x + sum.ln()
}

// ============================================================================
// Surface factory
// ============================================================================

/// Create all 4 loss surfaces
pub fn all_surfaces() -> Vec<Box<dyn LossSurface>> {
    let dim = 50; // 50-dimensional parameter space

    vec![
        Box::new(Rosenbrock::new(dim)),
        Box::new(Quadratic::new(dim)),
        Box::new(Rastrigin::new(dim)),
        Box::new(MNISTProxy::new(10)), // 10-class classifier
    ]
}

//! Benchmark runner and CSV writer
//!
//! Executes gradient optimization experiments across formats, surfaces, and seeds.
//! Outputs results to CSV files for analysis.

use super::surfaces::LossSurface;
use super::quantize::FormatQuantizer;
use super::metrics::{
    compute_grad_norm,
    compute_grad_variance,
    compute_snr,
    compute_bias_vs_fp32,
    BatchMetrics,
    ConvergenceTracker,
    StepMetrics,
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::path::Path;
use std::fs::File;
use csv::Writer;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub formats: Vec<String>,
    pub surfaces: Vec<Box<dyn LossSurface>>,
    pub seeds: Vec<u64>,
    pub steps_per_run: usize,
    pub learning_rate: f64,
    pub results_dir: String,
}

impl BenchmarkConfig {
    pub fn new(formats: Vec<String>, surfaces: Vec<Box<dyn LossSurface>>) -> Self {
        Self {
            formats,
            surfaces,
            seeds: crate::SEEDS.to_vec(),
            steps_per_run: crate::STEPS_PER_RUN,
            learning_rate: 0.01, // Default learning rate
            results_dir: "results".to_string(),
        }
    }
}

/// Result of a single benchmark run
#[derive(Debug)]
pub struct BenchmarkResult {
    pub format: String,
    pub surface: String,
    pub seed: u64,
    pub csv_path: String,
}

/// CSV writer for benchmark results
pub struct CSVWriter {
    writer: Writer<File>,
}

impl CSVWriter {
    pub fn new(path: &Path) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let mut writer = csv::Writer::from_writer(file);

        // Write header
        writer.write_record(&[
            "step",
            "loss",
            "grad_norm",
            "grad_var",
            "snr",
            "bias_vs_fp32",
        ])?;

        Ok(Self { writer })
    }

    pub fn write_row(&mut self, metrics: &StepMetrics) -> csv::Result<()> {
        self.writer.write_record(&[
            metrics.step.to_string(),
            format!("{:.6}", metrics.loss),
            format!("{:.6}", metrics.grad_norm),
            format!("{:.9}", metrics.grad_var),
            format!("{:.6}", metrics.snr),
            format!("{:.9}", metrics.bias_vs_fp32,
        ])
    }
}

/// Run single optimization for given format and surface
fn run_optimization<S: LossSurface, Q: FormatQuantizer>(
    surface: &S,
    quantizer: &Q,
    seed: u64,
    steps: usize,
    learning_rate: f64,
) -> std::io::Result<Vec<StepMetrics>> {
    let mut rng = StdRng::seed_from_u64(seed);

    // Initialize parameters
    let mut params: Vec<f64> = surface.sample_params(&mut rng);
    let dim = surface.dimensions();

    let mut history = Vec::with_capacity(steps);
    let mut params_fp32 = params.clone();

    for step in 0..steps {
        // Compute exact gradient
        let gradient_fp32: Vec<f64> = surface.compute_gradient(&params_fp32);

        // Quantize gradient in target format
        let gradient_quant: Vec<f64> = quantizer.quantize_batch(&gradient_fp32);

        // Compute FP32 metrics (for comparison)
        let batch_metrics = BatchMetrics::new(gradient_fp32.clone());
        let format_metrics = batch_metrics.compute_for_format(&gradient_quant);

        // Update parameters with quantized gradient
        for i in 0..dim {
            params[i] -= learning_rate * gradient_quant[i];
        }

        // Compute loss
        let loss = surface.compute_loss(&params);

        // Record metrics
        history.push(StepMetrics {
            step,
            loss,
            grad_norm: format_metrics.grad_norm,
            grad_var: format_metrics.grad_variance,
            snr: format_metrics.snr,
            bias_vs_fp32: format_metrics.bias_vs_fp32,
        });

        // Update FP32 params (for next step comparison)
        params_fp32 = params.clone();

        // Early stop if converged (loss < 1e-6)
        if loss < 1e-6 {
            break;
        }
    }

    Ok(history)
}

/// Run benchmark grid (formats × surfaces × seeds)
///
/// Total experiments: 12 × 4 × 5 = 240 runs
pub fn run_benchmark_grid(config: &BenchmarkConfig) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // Create results directory
    std::fs::create_dir_all(&config.results_dir)
        .expect("Failed to create results directory");

    for surface in &config.surfaces {
        let surface_name = surface.name();

        for seed in &config.seeds {
            for format_name in &config.formats {
                // Create CSV filename: gradient_metrics_{format}_{surface}_{seed}.csv
                let csv_filename = format!(
                    "gradient_metrics_{}_{}_{}.csv",
                    format_name, surface_name, seed
                );
                let csv_path = Path::new(&config.results_dir).join(&csv_filename);

                println!("Running: {} × {} × seed={}", format_name, surface_name, seed);

                // Get quantizer for this format
                let quantizer = super::quantize::create_quantizer(format_name)
                    .expect(&format!("Unknown format: {}", format_name));

                // Run optimization
                match run_optimization(
                    surface,
                    quantizer.as_ref(),
                    *seed,
                    config.steps_per_run,
                    config.learning_rate,
                ) {
                    Ok(history) => {
                        // Write to CSV
                        if let Ok(mut writer) = CSVWriter::new(&csv_path) {
                            for metrics in &history {
                                if let Err(e) = writer.write_row(metrics) {
                                    eprintln!("Failed to write CSV row: {}", e);
                                }
                            }
                            println!("  Wrote {} steps to {}", history.len(), csv_path);
                        }

                        results.push(BenchmarkResult {
                            format: format_name.to_string(),
                            surface: surface_name.to_string(),
                            seed: *seed,
                            csv_path: csv_path.display().to_string(),
                        });
                    }
                    Err(e) => {
                        eprintln!("Optimization failed: {}", e);
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_writer() {
        let temp = std::env::temp_dir();
        let path = temp.join("test.csv");
        let mut writer = CSVWriter::new(&path).unwrap();

        let metrics = StepMetrics {
            step: 1,
            loss: 100.0,
            grad_norm: 5.0,
            grad_var: 0.25,
            snr: 2.0,
            bias_vs_fp32: 0.5,
        };

        assert!(writer.write_row(&metrics).is_ok());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_benchmark_result() {
        let result = BenchmarkResult {
            format: "GF16".to_string(),
            surface: "Rosenbrock".to_string(),
            seed: 42,
            csv_path: "results/test.csv".to_string(),
        };

        assert_eq!(result.format, "GF16");
        assert_eq!(result.seed, 42);
    }
}

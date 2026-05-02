//! Main entry point for gf-gradient-benchmark
//!
//! Usage:
//!   cargo run -p gf-gradient-benchmark
//!   cargo run -p gf-gradient-benchmark -- --subset --formats GF16,GF8
//!   cargo run -p gf-gradient-benchmark -- --full-grid

use std::env;
use std::process::ExitCode;

mod surfaces;
mod quantize;
mod metrics;
mod runner;

use runner::run_benchmark_grid;
use runner::BenchmarkConfig;
use quantize::all_quantizers;
use surfaces::all_surfaces;
use crate::SEEDS;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--help" {
        print_help();
        return ExitCode::SUCCESS;
    }

    let config = BenchmarkConfig {
        formats: vec![
            "GF16".to_string(),
            "GF8".to_string(),
            "GF4".to_string(),
            "GF32".to_string(),
        ], // Default subset for quick testing
        surfaces: all_surfaces(),
        seeds: SEEDS.to_vec(),
        steps_per_run: 1000,
        learning_rate: 0.01,
        results_dir: "results".to_string(),
    };

    // Parse CLI args
    let mut full_grid = false;
    let mut formats_override: Option<Vec<String>> = None;

    for arg in &args {
        match arg.as_str() {
            "--full-grid" => {
                println!("Running full benchmark grid: 12 formats × 4 surfaces × 5 seeds = 240 runs");
                full_grid = true;
            }
            "--subset" => {
                // Use default subset
            }
            _ => {
                if arg.starts_with("--formats=") {
                    let formats = arg.strip_prefix("--formats=")
                        .unwrap()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    formats_override = Some(formats);
                }
            }
        }
    }

    // Apply overrides
    if let Some(formats) = formats_override {
        config.formats = formats;
    }

    if full_grid {
        config.formats = crate::ALL_FORMATS.iter().map(|s| s.to_string()).collect();
    }

    println!("=== GF Gradient Benchmark ===");
    println!("Formats: {}", config.formats.join(", "));
    println!("Surfaces: {}", config.surfaces.iter().map(|s| s.name()).collect::<Vec<_>>().join(", "));
    println!("Seeds: {:?}", config.seeds);
    println!("Steps per run: {}", config.steps_per_run);
    println!("Results dir: {}", config.results_dir);
    println!("================================");

    let results = run_benchmark_grid(&config);

    println!("\n=== Benchmark Complete ===");
    println!("Total runs: {}", results.len());
    println!("\nNext step: Run analysis script");
    println!("  cd crates/gf-gradient-benchmark");
    println!("  python scripts/analyze.py");

    ExitCode::SUCCESS
}

fn print_help() {
    println!("Usage: cargo run -p gf-gradient-benchmark [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --full-grid       Run all 12 formats × 4 surfaces × 5 seeds (240 runs)");
    println!("  --subset         Run default subset (quick test)");
    println!("  --formats=F1,F2   Run only specified formats");
    println!("  --help            Show this help message");
    println!();
    println!("Examples:");
    println!("  cargo run -p gf-gradient-benchmark --full-grid");
    println!("  cargo run -p gf-gradient-benchmark --subset --formats=GF16,GF8");
}

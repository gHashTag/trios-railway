use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::sync::OnceLock;

/// Install a panic hook that emits JSONL to stderr.
///
/// This ensures that panics during training are visible to the harvester
/// and CI tests. Without this hook, panic output goes to stderr
/// in an unstructured format that's hard to parse.
///
/// **Critical for catching "zero steps" failures** — if trainer panics
/// before first stdout flush, the panic line in stderr will be visible.
///
/// Output format:
/// ```json
/// {"event":"panic","msg":"thread 'main' panicked at '...' lib.rs:42"}
/// ```
pub fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    let _ = INSTALLED.set(());

    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();

        // Emit JSONL to stderr so CI can detect panics
        let panic_line = json!({
            "event": "panic",
            "msg": msg,
            "step": -1,  // -1 indicates panic before first step
        });

        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{}", panic_line);
        let _ = stderr.flush();

        // Call original hook for backtrace if configured
        original(info);
    }));
}

/// Configuration for smoke test experiments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeConfig {
    pub steps: u32,
    pub batch: u32,
    pub seed: u64,
    pub synthetic: bool,
    pub timeout_sec: u64,
}

impl Default for SmokeConfig {
    fn default() -> Self {
        Self {
            steps: 1,
            batch: 2,
            seed: 42,
            synthetic: true,
            timeout_sec: 30,
        }
    }
}

impl SmokeConfig {
    pub fn new(steps: u32) -> Self {
        Self {
            steps,
            ..Default::default()
        }
    }
}

/// A single training step output line in JSONL format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeStep {
    pub step: u32,
    pub loss: f64,
    pub bpb: Option<f64>,
    pub ts: String,
}

impl SmokeStep {
    pub fn new(step: u32, loss: f64, bpb: Option<f64>) -> Self {
        Self {
            step,
            loss,
            bpb,
            ts: Utc::now().to_rfc3339(),
        }
    }

    pub fn to_jsonl(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}

/// Trait for running a smoke test through the pipeline
pub trait SmokePipeline {
    fn run(&self) -> Result<Vec<SmokeStep>>;
}

/// Synthetic dataset generator for smoke testing
#[derive(Debug, Clone)]
pub struct SyntheticDataset {
    pub seed: u64,
    pub size: usize,
}

impl SyntheticDataset {
    pub fn new(seed: u64, size: usize) -> Self {
        Self { seed, size }
    }

    /// Generate a fake batch with deterministic "loss" values
    pub fn next_batch(&mut self, batch_size: u32) -> Vec<f64> {
        let mut losses = Vec::with_capacity(batch_size as usize);
        for i in 0..batch_size as usize {
            let loss = self.pseudo_random_loss(i);
            losses.push(loss);
        }
        losses
    }

    /// Deterministic pseudo-random loss based on seed and index
    fn pseudo_random_loss(&self, idx: usize) -> f64 {
        let x = (self.seed as usize + idx) as f64;
        let y = (x * 17.0) % 13.0;
        1.0 + y.abs() / 10.0
    }
}

/// Mock trainer that simulates training without GPU
#[derive(Debug, Clone)]
pub struct MockTrainer {
    pub config: SmokeConfig,
}

impl MockTrainer {
    pub fn new(config: SmokeConfig) -> Self {
        Self { config }
    }

    /// Run mock training steps and write JSONL to stdout
    pub fn run_with_stdout(&self) -> Result<()> {
        let mut dataset = SyntheticDataset::new(self.config.seed, 16);

        for step in 0..self.config.steps {
            let losses = dataset.next_batch(self.config.batch);
            let avg_loss: f64 = losses.iter().sum::<f64>() / losses.len() as f64;
            
            // Simulate BPB improvement over time
            let bpb = Some(3.0 - (avg_loss * 0.5).min(1.5));
            
            let step_data = SmokeStep::new(step, avg_loss, bpb);
            let line = step_data.to_jsonl()?;
            
            // Flush immediately - this is critical for harvester to see output
            writeln!(std::io::stdout(), "{}", line)?;
            std::io::stdout().flush()?;
        }

        Ok(())
    }

    /// Run and collect steps without stdout
    pub fn run_and_collect(&self) -> Result<Vec<SmokeStep>> {
        let mut dataset = SyntheticDataset::new(self.config.seed, 16);
        let mut steps = Vec::with_capacity(self.config.steps as usize);

        for step in 0..self.config.steps {
            let losses = dataset.next_batch(self.config.batch);
            let avg_loss: f64 = losses.iter().sum::<f64>() / losses.len() as f64;
            
            let bpb = Some(3.0 - (avg_loss * 0.5).min(1.5));
            steps.push(SmokeStep::new(step, avg_loss, bpb));
        }

        Ok(steps)
    }
}

impl SmokePipeline for MockTrainer {
    fn run(&self) -> Result<Vec<SmokeStep>> {
        self.run_and_collect()
    }
}

/// Parse JSONL line from stdout
pub fn parse_jsonl_line(line: &str) -> Result<SmokeStep> {
    let step: SmokeStep = serde_json::from_str(line)?;
    Ok(step)
}

/// Result of a local smoke test run
#[derive(Debug, Clone)]
pub struct SmokeResult {
    pub jsonl_lines: usize,
    pub samples: Vec<SmokeStep>,
}

/// Run a smoke test locally without stdout (in-memory collection)
///
/// This is useful for fast validation and CI testing without
/// requiring database or network connections.
pub fn run_local(config: &SmokeConfig) -> Result<SmokeResult> {
    let trainer = MockTrainer::new(config.clone());
    let steps = trainer.run()?;

    Ok(SmokeResult {
        jsonl_lines: steps.len(),
        samples: steps,
    })
}

/// Assert that at least one step was produced
pub fn assert_has_steps(steps: &[SmokeStep]) -> Result<()> {
    if steps.is_empty() {
        anyhow::bail!("No steps produced - trainer exited without output");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_config_default() {
        let config = SmokeConfig::default();
        assert_eq!(config.steps, 1);
        assert_eq!(config.batch, 2);
        assert_eq!(config.seed, 42);
        assert!(config.synthetic);
        assert_eq!(config.timeout_sec, 30);
    }

    #[test]
    fn test_smoke_step_jsonl() {
        let step = SmokeStep::new(0, 2.5, Some(3.0));
        let jsonl = step.to_jsonl().unwrap();
        
        let parsed: SmokeStep = serde_json::from_str(&jsonl).unwrap();
        assert_eq!(parsed.step, 0);
        assert!((parsed.loss - 2.5).abs() < 0.001);
        assert_eq!(parsed.bpb, Some(3.0));
    }

    #[test]
    fn test_synthetic_dataset_deterministic() {
        let mut ds1 = SyntheticDataset::new(42, 100);
        let mut ds2 = SyntheticDataset::new(42, 100);
        
        let batch1 = ds1.next_batch(4);
        let batch2 = ds2.next_batch(4);
        
        assert_eq!(batch1, batch2, "Same seed should produce same results");
    }

    #[test]
    fn test_mock_trainer_single_step() {
        let config = SmokeConfig::new(1);
        let trainer = MockTrainer::new(config.clone());
        
        let steps = trainer.run().unwrap();
        
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].step, 0);
        assert!(steps[0].loss > 0.0);
        assert!(steps[0].bpb.is_some());
    }

    #[test]
    fn test_mock_trainer_multi_step() {
        let config = SmokeConfig::new(5);
        let trainer = MockTrainer::new(config.clone());
        
        let steps = trainer.run().unwrap();
        
        assert_eq!(steps.len(), 5);
        for (i, step) in steps.iter().enumerate() {
            assert_eq!(step.step, i as u32);
            assert!(step.loss > 0.0);
        }
    }

    #[test]
    fn test_assert_has_steps_empty() {
        let steps: Vec<SmokeStep> = vec![];
        let result = assert_has_steps(&steps);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No steps produced"));
    }

    #[test]
    fn test_assert_has_steps_ok() {
        let steps = vec![SmokeStep::new(0, 2.5, None)];
        assert_has_steps(&steps).unwrap();
    }

    #[test]
    fn test_parse_jsonl_line() {
        let json = r#"{"step":0,"loss":2.5,"bpb":3.0,"ts":"2024-01-01T00:00:00+00:00"}"#;
        let step = parse_jsonl_line(json).unwrap();
        
        assert_eq!(step.step, 0);
        assert!((step.loss - 2.5).abs() < 0.001);
        assert_eq!(step.bpb, Some(3.0));
    }
}

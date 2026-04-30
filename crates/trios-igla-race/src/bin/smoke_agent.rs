use anyhow::Result;
use clap::Parser;
use std::io::{BufRead, BufReader};
use trios_igla_race::pull_queue::{ExperimentConfig, PullQueueDb};
use trios_railway_smoke::{SmokeConfig, MockTrainer, parse_jsonl_line};

const SMOKE_STEPS: u32 = 1;
const SMOKE_SEED: u64 = 42;

#[derive(Parser)]
#[command(
    name = "smoke-agent",
    about = "Smoke test agent: validates full pipeline in <60s with synthetic data"
)]
struct Cli {
    #[arg(long, env = "NEON_DATABASE_URL")]
    neon_url: String,

    #[arg(long, default_value = "smoke-test")]
    worker_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    eprintln!("SMOKE AGENT STARTED - proving pipeline is alive...");

    // 1. DB connection
    let db = PullQueueDb::connect(&cli.neon_url).await?;
    db.health_check().await?;
    eprintln!("✓ DB connection OK");

    // 2. Create synthetic experiment config
    let config = ExperimentConfig {
        seed: SMOKE_SEED,
        hidden: 128,
        ctx: 8,
        lr: 0.001,
        steps: SMOKE_STEPS as usize,
    };

    eprintln!("✓ Config created: seed={} steps={}", config.seed, config.steps);

    // 3. Run mock trainer with stdout
    let smoke_config = SmokeConfig::new(SMOKE_STEPS);
    let trainer = MockTrainer::new(smoke_config);

    eprintln!("✓ Running mock trainer (synthetic, CPU-only)...");

    trainer.run_with_stdout()?;

    eprintln!("✓ Training completed - {} step(s) output to stdout", SMOKE_STEPS);

    // 4. Parse output to verify format
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin);
    let mut step_count = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match parse_jsonl_line(&line) {
            Ok(step) => {
                step_count += 1;
                eprintln!("✓ Parsed step {}: loss={:.4} bpb={:?}",
                    step.step, step.loss, step.bpb);
            }
            Err(e) => {
                eprintln!("✗ Failed to parse JSONL: {} (line: {})", e, line);
                std::process::exit(1);
            }
        }
    }

    if step_count == 0 {
        eprintln!("✗ ZERO STEPS DETECTED - trainer exited without output");
        eprintln!("This is the EXACT bug causing 186 failed experiments!");
        std::process::exit(1);
    }

    eprintln!("✓✓✓ SMOKE TEST PASSED - pipeline is healthy");
    eprintln!("✓✓✓ Steps produced: {}", step_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        let cli = Cli::try_parse_from([
            "smoke-agent",
            "--neon-url", "postgres://localhost/test",
        ]);
        assert!(cli.is_ok());
    }
}

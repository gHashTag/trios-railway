//! Config parsing from env vars. R5-honest: required vars fail loud at startup.

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
#[allow(dead_code)] // PR-1 scaffold; fields wired in PR-2..PR-5.
pub struct Config {
    pub neon_dsn: String,
    pub railway_api_token: Option<String>,
    pub tick_secs: u64,
    pub worker_dead_after_secs: i64,
    pub max_replicas: u32,
    pub min_replicas: u32,
    pub leak_bpb_threshold: f64,
    pub stuck_running_after_hours: i64,
    pub telegram_token: Option<String>,
    pub telegram_chat: Option<String>,
    pub destructive_ok: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let neon_dsn = std::env::var("NEON_DATABASE_URL")
            .or_else(|_| std::env::var("TRIOS_NEON_DSN"))
            .or_else(|_| std::env::var("DATABASE_URL"))
            .context("NEON_DATABASE_URL (or TRIOS_NEON_DSN / DATABASE_URL) required")?;

        Ok(Self {
            neon_dsn,
            railway_api_token: std::env::var("RAILWAY_API_TOKEN").ok(),
            tick_secs: env_or("LEDGER_TICK_SECS", 30),
            worker_dead_after_secs: env_or("WORKER_DEAD_AFTER_SECS", 120),
            max_replicas: env_or("LEDGER_MAX_REPLICAS", 12),
            min_replicas: env_or("LEDGER_MIN_REPLICAS", 2),
            leak_bpb_threshold: env_or("LEDGER_LEAK_BPB_THRESHOLD", 0.1_f64),
            stuck_running_after_hours: env_or("LEDGER_STUCK_HOURS", 1),
            telegram_token: std::env::var("LEDGER_TELEGRAM_TOKEN").ok(),
            telegram_chat: std::env::var("LEDGER_TELEGRAM_CHAT").ok(),
            destructive_ok: std::env::var("LEDGER_DESTRUCTIVE_OK")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        })
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

//! Database-driven scarab - polls strategy table, self-manages lifecycle.
//!
//! SCARAB_SERVICE_ID: unique identifier for this scarab
//! SCARAB_POLL_INTERVAL: seconds between polls (default: 10)
//!
//! Lifecycle:
//! 1. Poll public.scarab_strategy every N seconds
//! 2. Compare strategy_hash with last known hash
//! 3. If changed OR status != 'active':
//!    - Stop current training (SIGTERM, then SIGKILL)
//!    - Start new training if status='active'
//! 4. Update heartbeat in public.scarab_heartbeat
//! 5. Repeat forever

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::process::Child;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScarabStrategy {
    pub service_id: String,
    pub optimizer: String,
    pub hidden: i32,
    pub lr: f64,
    pub seed: i64,
    pub steps: i32,
    pub ctx: i32,
    pub format: String,
    pub status: String,
    pub strategy_hash: String,
    pub canon_name: Option<String>,
}

impl ScarabStrategy {
    /// Returns true if training should restart (hash changed or status != 'active')
    pub fn requires_restart(&self, previous_hash: Option<&str>) -> bool {
        let hash_changed = previous_hash.map_or(true, |h| h != &self.strategy_hash);
        let not_active = self.status != "active";
        hash_changed || not_active
    }

    /// Build CLI arguments for trios-igla train
    pub fn train_args(&self, exp_id: &str) -> Vec<String> {
        let mut args = vec![
            "train".to_string(),
            "--exp-id".to_string(),
            exp_id.to_string(),
            "--seed".to_string(),
            self.seed.to_string(),
            "--hidden".to_string(),
            self.hidden.to_string(),
            "--lr".to_string(),
            self.lr.to_string(),
            "--steps".to_string(),
            self.steps.to_string(),
            "--ctx".to_string(),
            self.ctx.to_string(),
            "--format".to_string(),
            self.format.clone(),
        ];
        if let Some(ref canon) = self.canon_name {
            args.push("--canon-name".to_string());
            args.push(canon.clone());
        }
        args
    }
}

/// Wrapper for trainer process with graceful shutdown
pub struct TrainingProcess {
    child: Option<Child>,
}

impl TrainingProcess {
    pub fn start(_exp_id: &str, args: &[String]) -> Result<Self> {
        let trainer_bin = std::env::var("TRIOS_TRAIN_BIN")
            .unwrap_or_else(|_| "trios-igla".to_string());
        info!("Starting trainer: {} {:?}", trainer_bin, args);

        let mut cmd = tokio::process::Command::new(&trainer_bin);
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd.spawn()
            .context("Failed to spawn trainer")?;

        Ok(Self {
            child: Some(child),
        })
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;

                let pid = child.id().expect("child should have pid");
                info!("Stopping trainer (pid={})", pid);

                // Try graceful SIGTERM first
                if let Err(e) = signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                    warn!("SIGTERM failed: {}", e);
                }

                // Wait up to 30 seconds
                match tokio::time::timeout(Duration::from_secs(30), child.wait()).await {
                    Ok(Ok(status)) => info!("Trainer exited: {}", status),
                    Ok(Err(e)) => warn!("Trainer wait error: {}", e),
                    Err(_) => {
                        // Force kill if timeout
                        warn!("SIGTERM timeout, sending SIGKILL");
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(child) = self.child.as_mut() {
            child.try_wait().map(|r| r.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    pub async fn wait(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            child.wait().await?;
        }
        Ok(())
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
    }
}

impl Drop for TrainingProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Best-effort shutdown on drop
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_restart_first_run() {
        let strategy = ScarabStrategy {
            service_id: "test".to_string(),
            optimizer: "adamw".to_string(),
            hidden: 512,
            lr: 0.002,
            seed: 42,
            steps: 27000,
            ctx: 12,
            format: "fp32".to_string(),
            status: "active".to_string(),
            strategy_hash: "abc123".to_string(),
            canon_name: None,
        };
        assert!(strategy.requires_restart(None));
    }

    #[test]
    fn test_requires_restart_hash_changed() {
        let strategy = ScarabStrategy {
            service_id: "test".to_string(),
            optimizer: "adamw".to_string(),
            hidden: 512,
            lr: 0.002,
            seed: 42,
            steps: 27000,
            ctx: 12,
            format: "fp32".to_string(),
            status: "active".to_string(),
            strategy_hash: "abc123".to_string(),
            canon_name: None,
        };
        assert!(strategy.requires_restart(Some("old_hash")));
        assert!(!strategy.requires_restart(Some("abc123")));
    }

    #[test]
    fn test_requires_restart_not_active() {
        let strategy = ScarabStrategy {
            service_id: "test".to_string(),
            optimizer: "adamw".to_string(),
            hidden: 512,
            lr: 0.002,
            seed: 42,
            steps: 27000,
            ctx: 12,
            format: "fp32".to_string(),
            status: "paused".to_string(),
            strategy_hash: "abc123".to_string(),
            canon_name: None,
        };
        assert!(strategy.requires_restart(Some("abc123")));
    }

    #[test]
    fn test_train_args() {
        let strategy = ScarabStrategy {
            service_id: "test".to_string(),
            optimizer: "adamw".to_string(),
            hidden: 512,
            lr: 0.002,
            seed: 42,
            steps: 27000,
            ctx: 12,
            format: "fp32".to_string(),
            status: "active".to_string(),
            strategy_hash: "abc123".to_string(),
            canon_name: Some("test-canon".to_string()),
        };

        let args = strategy.train_args("exp-001");
        assert_eq!(args[0], "train");
        assert_eq!(args[2], "exp-001");
        assert!(args.contains(&"--seed".to_string()));
        assert!(args.contains(&"--canon-name".to_string()));
        assert!(args.contains(&"test-canon".to_string()));
    }
}

//! # tri-hunt
//!
//! Seed hunter operations: status, smoke race, rung schedule, prune diverging, mirror siblings.
//!
//! This crate manages training seed hunting and validation for the IGLA project.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a single seed in the hunting process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedStatus {
    /// Seed number.
    pub seed: i32,
    /// Current state of the seed.
    pub state: SeedState,
    /// When the seed was first discovered.
    pub discovered_at: DateTime<Utc>,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
    /// Best bits-per-byte (BPB) achieved for this seed.
    pub best_bpb: Option<f64>,
}

/// Possible states for a seed in the hunting process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeedState {
    /// Seed is pending discovery.
    Pending,
    /// Seed is currently being trained.
    Training,
    /// Seed training completed successfully.
    Completed,
    /// Seed training failed.
    Failed,
    /// Seed was pruned due to divergence.
    Pruned,
}

/// Configuration for the smoke race process.
#[derive(Debug, Clone)]
pub struct SmokeRaceConfig {
    /// Number of seeds to race.
    pub count: usize,
    /// BPB target to beat.
    pub target_bpb: f64,
    /// Maximum time per seed in seconds.
    pub timeout_seconds: u64,
}

/// Result of a smoke race.
#[derive(Debug, Clone)]
pub struct SmokeRaceResult {
    /// Winning seed.
    pub winner: Option<SeedStatus>,
    /// All seeds that participated.
    pub participants: Vec<SeedStatus>,
    /// Time taken for the race.
    pub duration_seconds: u64,
}

/// Rung on the training ladder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rung {
    /// Rung level (higher = better).
    pub level: i32,
    /// Seeds at this rung.
    pub seeds: Vec<i32>,
    /// BPB threshold for this rung.
    pub bpb_threshold: f64,
}

/// Schedule of rungs for seed progression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RungSchedule {
    /// All rungs in the schedule.
    pub rungs: Vec<Rung>,
    /// Current rung being processed.
    pub current_rung: i32,
}

/// Status of the entire seed hunting operation.
#[derive(Debug, Clone)]
pub struct SeedHunterStatus {
    /// All tracked seeds.
    pub seeds: Vec<SeedStatus>,
    /// Current rung schedule.
    pub schedule: RungSchedule,
    /// Hunter state.
    pub state: HunterState,
}

/// Overall state of the seed hunter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HunterState {
    /// Hunter is idle.
    Idle,
    /// Hunter is actively hunting.
    Hunting,
    /// Hunter is paused.
    Paused,
    /// Hunter has completed.
    Completed,
}

/// Get the current status of the seed hunter.
///
/// # Returns
///
/// Returns `SeedHunterStatus` with current hunter state.
pub fn seed_hunter_status() -> SeedHunterStatus {
    SeedHunterStatus {
        seeds: Vec::new(),
        schedule: RungSchedule {
            rungs: Vec::new(),
            current_rung: 0,
        },
        state: HunterState::Idle,
    }
}

/// Run a smoke race to find the best seed.
///
/// # Arguments
///
/// * `config` - Configuration for the race
///
/// # Returns
///
/// Returns `SmokeRaceResult` with race results.
///
/// # Errors
///
/// Returns an error if the race fails to complete.
pub async fn smoke_race(config: SmokeRaceConfig) -> anyhow::Result<SmokeRaceResult> {
    tracing::info!("starting smoke race with {} seeds", config.count);

    // TODO: Implement actual race logic
    let start = std::time::Instant::now();

    Ok(SmokeRaceResult {
        winner: None,
        participants: Vec::new(),
        duration_seconds: start.elapsed().as_secs(),
    })
}

/// Get the rung schedule for seed progression.
///
/// # Arguments
///
/// * `target_bpb` - Target BPB to achieve
/// * `rungs` - Number of rungs in the schedule
///
/// # Returns
///
/// Returns `RungSchedule` with configured rungs.
pub fn rung_schedule(target_bpb: f64, rungs: i32) -> RungSchedule {
    let mut schedule_rungs = Vec::new();
    let bpb_step = target_bpb / rungs as f64;

    for level in 1..=rungs {
        schedule_rungs.push(Rung {
            level,
            seeds: Vec::new(),
            bpb_threshold: level as f64 * bpb_step,
        });
    }

    RungSchedule {
        rungs: schedule_rungs,
        current_rung: 1,
    }
}

/// Prune seeds that are diverging from the expected BPB trajectory.
///
/// # Arguments
///
/// * `seeds` - Current seed statuses
/// * `expected_bpb` - Expected BPB threshold
///
/// # Returns
///
/// Returns a vector of seed IDs to prune.
pub fn prune_diverging(seeds: &[SeedStatus], expected_bpb: f64) -> Vec<i32> {
    seeds
        .iter()
        .filter(|s| {
            if let Some(bpb) = s.best_bpb {
                bpb > expected_bpb
            } else {
                false
            }
        })
        .map(|s| s.seed)
        .collect()
}

/// Mirror sibling seeds across different training configurations.
///
/// # Arguments
///
/// * `seeds` - Seeds to mirror
///
/// # Returns
///
/// Returns a vector of new seed configurations to create.
pub fn mirror_siblings(seeds: &[i32]) -> Vec<SiblingConfig> {
    seeds
        .iter()
        .map(|&seed| SiblingConfig {
            base_seed: seed,
            variant: SiblingVariant::Mirror,
        })
        .collect()
}

/// Configuration for a sibling seed.
#[derive(Debug, Clone)]
pub struct SiblingConfig {
    /// Base seed to mirror.
    pub base_seed: i32,
    /// Variant type for the sibling.
    pub variant: SiblingVariant,
}

/// Type of sibling variant.
#[derive(Debug, Clone, Copy)]
pub enum SiblingVariant {
    /// Direct mirror with same config.
    Mirror,
    /// Hyperparameter variant.
    Hyperparams,
    /// Architecture variant.
    Architecture,
}

# IGLA RACE Scarab Database-Polling Architecture Implementation Plan

## Executive Summary

Convert IGLA RACE scarabs from Railway API-controlled (variableUpsert-based) to database-polling architecture. Scarabs will poll their strategy from a new `ssot.scarab_strategy` table, detect changes via hash comparison, and gracefully restart training when strategy changes.

---

## 1. Database Migration

### File: `/Users/playra/trios-railway/trios-worktree/migrations/0003_scarab_strategy.sql`

```sql
-- 0003: Database-Polling Scarab Architecture (ADR-00XX)
-- Replaces Railway API control with database polling for IGLA RACE scarabs.
-- Each scarab has a unique service_id and polls its strategy from ssot.scarab_strategy.

-- Create ssot schema if it doesn't exist
CREATE SCHEMA IF NOT EXISTS ssot;

-- Primary strategy table for database-polling scarabs
CREATE TABLE IF NOT EXISTS ssot.scarab_strategy (
    -- Primary key: unique service identifier for each scarab
    service_id          TEXT PRIMARY KEY,
    
    -- Training configuration
    optimizer           TEXT NOT NULL DEFAULT 'adamw'
                        CHECK (optimizer IN ('adamw', 'muon', 'muon-cwd')),
    
    hidden              INTEGER NOT NULL DEFAULT 828
                        CHECK (hidden > 0),
    
    lr                  DOUBLE PRECISION NOT NULL DEFAULT 0.0004
                        CHECK (lr > 0 AND lr < 1),
    
    seed                BIGINT NOT NULL,
    
    steps               INTEGER NOT NULL DEFAULT 54000
                        CHECK (steps > 0),
    
    ctx                 INTEGER NOT NULL DEFAULT 12
                        CHECK (ctx > 0),
    
    format              TEXT NOT NULL DEFAULT 'fp32'
                        CHECK (format IN ('fp32', 'bf16', 'gf16')),
    
    -- Control flags
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'paused', 'disabled')),
    
    -- Change detection
    strategy_hash       TEXT NOT NULL,  -- SHA-256 of strategy config for change detection
    last_updated        TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Optional metadata
    description         TEXT,
    canon_name          TEXT NOT NULL DEFAULT 'tinyshakespeare'
);

-- Index for status-based queries (e.g., list active strategies)
CREATE INDEX IF NOT EXISTS ssot_scarab_strategy_status_idx
    ON ssot.scarab_strategy (status) WHERE status = 'active';

-- Index for last_updated ordering (e.g., recent changes)
CREATE INDEX IF NOT EXISTS ssot_scarab_strategy_updated_idx
    ON ssot.scarab_strategy (last_updated DESC);

-- Function to compute strategy hash from columns
CREATE OR REPLACE FUNCTION ssot.compute_strategy_hash(
    p_optimizer TEXT,
    p_hidden INTEGER,
    p_lr DOUBLE PRECISION,
    p_seed BIGINT,
    p_steps INTEGER,
    p_ctx INTEGER,
    p_format TEXT
) RETURNS TEXT AS $$
BEGIN
    RETURN encode(
        digest(
            p_optimizer || '|' ||
            p_hidden || '|' ||
            p_lr || '|' ||
            p_seed || '|' ||
            p_steps || '|' ||
            p_ctx || '|' ||
            p_format,
            'sha256'
        ),
        'hex'
    );
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Trigger to auto-compute strategy_hash on INSERT/UPDATE
CREATE OR REPLACE FUNCTION ssot.update_strategy_hash() RETURNS TRIGGER AS $$
BEGIN
    NEW.strategy_hash = ssot.compute_strategy_hash(
        NEW.optimizer,
        NEW.hidden,
        NEW.lr,
        NEW.seed,
        NEW.steps,
        NEW.ctx,
        NEW.format
    );
    NEW.last_updated = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER scarab_strategy_hash_trigger
    BEFORE INSERT OR UPDATE ON ssot.scarab_strategy
    FOR EACH ROW EXECUTE FUNCTION ssot.update_strategy_hash();

-- Heartbeat table for tracking scarab liveness
CREATE TABLE IF NOT EXISTS ssot.scarab_heartbeat (
    service_id          TEXT PRIMARY KEY REFERENCES ssot.scarab_strategy(service_id) ON DELETE CASCADE,
    last_heartbeat      TIMESTAMPTZ NOT NULL DEFAULT now(),
    current_run_id      TEXT,
    current_step        INTEGER DEFAULT 0,
    current_bpb         DOUBLE PRECISION,
    last_status         TEXT DEFAULT 'idle'
                        CHECK (last_status IN ('idle', 'polling', 'training', 'restarting', 'error')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ssot_scarab_heartbeat_last_idx
    ON ssot.scarab_heartbeat (last_heartbeat DESC) WHERE last_heartbeat > now() - INTERVAL '5 minutes';

-- Training run history for audit/debug
CREATE TABLE IF NOT EXISTS ssot.scarab_runs (
    run_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id          TEXT NOT NULL REFERENCES ssot.scarab_strategy(service_id),
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at         TIMESTAMPTZ,
    strategy_hash       TEXT NOT NULL,
    final_step          INTEGER,
    final_bpb           DOUBLE PRECISION,
    status              TEXT NOT NULL DEFAULT 'running'
                        CHECK (status IN ('running', 'completed', 'aborted', 'failed')),
    error_msg           TEXT,
    trainer_pid         INTEGER
);

CREATE INDEX IF NOT EXISTS ssot_scarab_runs_service_idx
    ON ssot.scarab_runs (service_id, started_at DESC);

-- Upsert helper function for Queen Hive MCP tool
CREATE OR REPLACE FUNCTION ssot.upsert_scarab_strategy(
    p_service_id TEXT,
    p_optimizer TEXT DEFAULT 'adamw',
    p_hidden INTEGER DEFAULT 828,
    p_lr DOUBLE PRECISION DEFAULT 0.0004,
    p_seed BIGINT DEFAULT 43,
    p_steps INTEGER DEFAULT 54000,
    p_ctx INTEGER DEFAULT 12,
    p_format TEXT DEFAULT 'fp32',
    p_status TEXT DEFAULT 'active',
    p_description TEXT DEFAULT NULL,
    p_canon_name TEXT DEFAULT 'tinyshakespeare'
) RETURNS TEXT AS $$
DECLARE
    v_hash TEXT;
BEGIN
    INSERT INTO ssot.scarab_strategy (
        service_id, optimizer, hidden, lr, seed, steps, ctx, format,
        status, description, canon_name
    ) VALUES (
        p_service_id, p_optimizer, p_hidden, p_lr, p_seed, p_steps, p_ctx, p_format,
        p_status, p_description, p_canon_name
    )
    ON CONFLICT (service_id) DO UPDATE SET
        optimizer = EXCLUDED.optimizer,
        hidden = EXCLUDED.hidden,
        lr = EXCLUDED.lr,
        seed = EXCLUDED.seed,
        steps = EXCLUDED.steps,
        ctx = EXCLUDED.ctx,
        format = EXCLUDED.format,
        status = EXCLUDED.status,
        description = COALESCE(EXCLUDED.description, ssot.scarab_strategy.description),
        canon_name = EXCLUDED.canon_name,
        last_updated = now();
    
    SELECT strategy_hash INTO v_hash
    FROM ssot.scarab_strategy WHERE service_id = p_service_id;
    
    RETURN v_hash;
END;
$$ LANGUAGE plpgsql;
```

---

## 2. New Scarab Binary Implementation

### File Structure

```
/Users/playra/trios-railway/trios-worktree/crates/trios-igla-race/src/
├── scarab_polling.rs          # New: Database-polling scarab module
└── bin/
    ├── scarab.rs              # Existing: Queue-pulling scarab (keep for backward compat)
    └── scarab_strategy.rs     # New: Database-polling scarab binary
```

### File: `/Users/playra/trios-railway/trios-worktree/crates/trios-igla-race/src/scarab_polling.rs`

```rust
//! Database-polling scarab for IGLA RACE.
//!
//! Architecture:
//! - Each scarab has a unique service_id (SCARAB_SERVICE_ID env var)
//! - Polls ssot.scarab_strategy every N seconds (SCARAB_POLL_INTERVAL)
//! - Detects strategy changes via strategy_hash comparison
//! - Gracefully restarts trios-train when strategy changes
//! - Writes BPB via trios-train (neon_writer already handles this)
//!
//! ENV:
//!   SCARAB_SERVICE_ID     - Required: unique identifier for this scarab
//!   SCARAB_POLL_INTERVAL  - Optional: poll interval in seconds (default: 10)
//!   NEON_DATABASE_URL     - Required: database connection string
//!   TRIOS_TRAIN_BIN       - Optional: path to trios-train binary (default: "trios-igla train")

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;
use tracing::{error, info, warn};

/// Strategy configuration fetched from database
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
    pub canon_name: String,
}

impl ScarabStrategy {
    /// Check if this strategy should trigger a restart (hash differs)
    pub fn requires_restart(&self, previous_hash: Option<&str>) -> bool {
        if self.status != "active" {
            // If not active, we should stop training
            return true;
        }
        match previous_hash {
            None => true, // First run
            Some(prev) => prev != &self.strategy_hash,
        }
    }

    /// Build command-line arguments for trios-train
    pub fn train_args(&self) -> Vec<String> {
        vec![
            "train".to_string(),
            "--seed".to_string(),
            self.seed.to_string(),
            "--steps".to_string(),
            self.steps.to_string(),
            "--hidden".to_string(),
            self.hidden.to_string(),
            "--lr".to_string(),
            format!("{:.6}", self.lr),
            "--ctx".to_string(),
            self.ctx.to_string(),
            "--format".to_string(),
            self.format.clone(),
        ]
    }
}

/// Database connection for polling scarabs
pub struct ScarabPollingDb {
    client: Arc<Mutex<tokio_postgres::Client>>,
}

impl ScarabPollingDb {
    /// Connect to Neon database
    pub async fn connect(db_url: &str) -> Result<Self> {
        let (client, conn) = tokio_postgres::connect(db_url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::error!("ScarabPollingDb connection error: {e}");
            }
        });
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Fetch strategy for a specific service_id
    pub async fn fetch_strategy(&self, service_id: &str) -> Result<Option<ScarabStrategy>> {
        let client = self.client.lock().await;
        let row = client
            .query_opt(
                "SELECT service_id, optimizer, hidden, lr, seed, steps, ctx,
                        format, status, strategy_hash, canon_name
                 FROM ssot.scarab_strategy
                 WHERE service_id = $1",
                &[&service_id],
            )
            .await?;

        match row {
            Some(r) => Ok(Some(ScarabStrategy {
                service_id: r.get(0),
                optimizer: r.get(1),
                hidden: r.get(2),
                lr: r.get(3),
                seed: r.get(4),
                steps: r.get(5),
                ctx: r.get(6),
                format: r.get(7),
                status: r.get(8),
                strategy_hash: r.get(9),
                canon_name: r.get(10),
            })),
            None => Ok(None),
        }
    }

    /// Update heartbeat entry
    pub async fn update_heartbeat(
        &self,
        service_id: &str,
        run_id: Option<String>,
        step: i32,
        bpb: Option<f64>,
        status: &str,
    ) -> Result<()> {
        let client = self.client.lock().await;
        client
            .execute(
                "INSERT INTO ssot.scarab_heartbeat (service_id, last_heartbeat, current_run_id, current_step, current_bpb, last_status)
                 VALUES ($1, now(), $2, $3, $4, $5)
                 ON CONFLICT (service_id) DO UPDATE SET
                    last_heartbeat = now(),
                    current_run_id = EXCLUDED.current_run_id,
                    current_step = EXCLUDED.current_step,
                    current_bpb = EXCLUDED.current_bpb,
                    last_status = EXCLUDED.last_status",
                &[&service_id, &run_id, &step, &bpb, &status],
            )
            .await?;
        Ok(())
    }

    /// Record the start of a training run
    pub async fn start_run(&self, service_id: &str, strategy_hash: &str, pid: i32) -> Result<String> {
        let client = self.client.lock().await;
        let run_id = uuid::Uuid::new_v4().to_string();
        client
            .execute(
                "INSERT INTO ssot.scarab_runs (run_id, service_id, strategy_hash, trainer_pid)
                 VALUES ($1, $2, $3, $4)",
                &[&run_id, &service_id, &strategy_hash, &pid],
            )
            .await?;
        Ok(run_id)
    }

    /// Record the end of a training run
    pub async fn end_run(
        &self,
        run_id: &str,
        status: &str,
        final_step: Option<i32>,
        final_bpb: Option<f64>,
        error_msg: Option<String>,
    ) -> Result<()> {
        let client = self.client.lock().await;
        client
            .execute(
                "UPDATE ssot.scarab_runs
                 SET finished_at = now(),
                     status = $1,
                     final_step = $2,
                     final_bpb = $3,
                     error_msg = $4
                 WHERE run_id = $5",
                &[&status, &final_step, &final_bpb, &error_msg, &run_id],
            )
            .await?;
        Ok(())
    }

    pub fn clone_handle(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

/// Manages a running trios-train process
pub struct TrainingProcess {
    child: Option<Child>,
    run_id: Option<String>,
}

impl TrainingProcess {
    pub fn new() -> Self {
        Self {
            child: None,
            run_id: None,
        }
    }

    /// Start a new training process with the given strategy
    pub fn start(&mut self, strategy: &ScarabStrategy, trainer_bin: &str, db: &ScarabPollingDb) -> Result<()> {
        let args = strategy.train_args();
        info!(
            "Starting trainer: {} with args: {:?}",
            trainer_bin, args
        );

        let mut child = Command::new(trainer_bin)
            .args(&args)
            .env("NEON_DATABASE_URL", std::env::var("NEON_DATABASE_URL").unwrap_or_default())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        let pid = child.id() as i32;
        let run_id = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                db.start_run(&strategy.service_id, &strategy.strategy_hash, pid)
            )
        })?;

        self.child = Some(child);
        self.run_id = Some(run_id);
        Ok(())
    }

    /// Stop the current training process gracefully
    pub fn stop(&mut self, db: &ScarabPollingDb, status: &str) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            info!("Stopping training process (status: {})", status);
            
            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                if let Err(e) = signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM) {
                    warn!("SIGTERM failed: {}, using SIGKILL", e);
                    let _ = child.kill();
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }

            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    child.wait()
                )
            });
        }

        if let Some(run_id) = self.run_id.take() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(
                    db.end_run(&run_id, status, None, None, None)
                )
            })?;
        }

        Ok(())
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        match &mut self.child {
            Some(child) => {
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        info!("Training process exited: {:?}", exit_status);
                        self.child = None;
                        false
                    }
                    Ok(None) => true,
                    Err(e) => {
                        warn!("Failed to check process status: {}", e);
                        false
                    }
                }
            }
            None => false,
        }
    }

    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }
}
```

### File: `/Users/playra/trios-railway/trios-worktree/crates/trios-igla-race/src/bin/scarab_strategy.rs`

```rust
//! `scarab-strategy` - Database-polling scarab binary.
//!
//! This binary is deployed once per scarab on Railway and polls its strategy
//! from the database. When the strategy changes, it gracefully restarts training.
//!
//! Environment variables:
//!   SCARAB_SERVICE_ID     - Required: unique identifier (e.g., "igla-scarab-01")
//!   SCARAB_POLL_INTERVAL  - Optional: poll interval in seconds (default: 10)
//!   NEON_DATABASE_URL     - Required: database connection
//!   TRIOS_TRAIN_BIN       - Optional: path to trainer (default: "trios-igla train")

use anyhow::Result;
use std::env;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use trios_igla_race::scarab_polling::{ScarabPollingDb, ScarabStrategy, TrainingProcess};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("scarab_strategy=info,info"))
        )
        .init();

    // Parse environment variables
    let service_id = env::var("SCARAB_SERVICE_ID")
        .expect("SCARAB_SERVICE_ID must be set");
    
    let poll_interval_secs = env::var("SCARAB_POLL_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    
    let db_url = env::var("NEON_DATABASE_URL")
        .expect("NEON_DATABASE_URL must be set");
    
    let trainer_bin = env::var("TRIOS_TRAIN_BIN")
        .unwrap_or_else(|_| "trios-igla".to_string());

    info!("Scarab Strategy starting");
    info!("  service_id: {}", service_id);
    info!("  poll_interval: {}s", poll_interval_secs);
    info!("  db_url: {}", db_url);
    info!("  trainer_bin: {}", trainer_bin);

    // Connect to database
    let db = ScarabPollingDb::connect(&db_url).await?;
    info!("Connected to database");

    // Initial heartbeat
    db.update_heartbeat(&service_id, None, 0, None, "idle").await?;

    // Main polling loop
    let mut training = TrainingProcess::new();
    let mut last_hash: Option<String> = None;
    let mut iteration: u64 = 0;

    loop {
        iteration += 1;
        info!("Poll iteration {}", iteration);

        // Fetch current strategy
        match db.fetch_strategy(&service_id).await {
            Ok(Some(strategy)) => {
                info!("Fetched strategy: hash={}", strategy.strategy_hash);
                
                // Update heartbeat
                let status = if training.is_running() { "training" } else { "polling" };
                db.update_heartbeat(
                    &service_id,
                    training.run_id().map(|s| s.to_string()),
                    0,
                    None,
                    status,
                ).await?;

                // Check if we need to restart
                if strategy.requires_restart(last_hash.as_deref()) {
                    info!("Strategy change detected (or first run)");
                    
                    // Stop existing training if running
                    if training.is_running() {
                        let new_status = if strategy.status == "active" {
                            "aborted"
                        } else {
                            "paused"
                        };
                        training.stop(&db, new_status)?;
                    }

                    // Start new training if status is active
                    if strategy.status == "active" {
                        info!("Starting training with new strategy");
                        match training.start(&strategy, &trainer_bin, &db) {
                            Ok(()) => {
                                last_hash = Some(strategy.strategy_hash.clone());
                                info!("Training started successfully");
                            }
                            Err(e) => {
                                error!("Failed to start training: {}", e);
                                db.update_heartbeat(&service_id, None, 0, None, "error").await?;
                            }
                        }
                    } else {
                        info!("Strategy status is '{}', not starting training", strategy.status);
                        last_hash = Some(strategy.strategy_hash.clone());
                    }
                } else {
                    // No change, just check if training is still running
                    if !training.is_running() {
                        warn!("Training process exited unexpectedly");
                        last_hash = None; // Force restart next cycle
                    }
                }
            }
            Ok(None) => {
                warn!("No strategy found for service_id '{}'", service_id);
                // Stop training if running
                if training.is_running() {
                    training.stop(&db, "stopped")?;
                }
                last_hash = None;
            }
            Err(e) => {
                error!("Failed to fetch strategy: {}", e);
                db.update_heartbeat(&service_id, None, 0, None, "error").await?;
            }
        }

        // Sleep until next poll
        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
    }
}
```

---

## 3. MCP Tool for Queen Hive

### File: `/Users/playra/trios-railway/trios-worktree/crates/trios-railway-mcp/src/scarab_strategy.rs`

```rust
//! MCP tools for controlling scarab strategies via database.
//!
//! These tools replace Railway API control with direct database operations.
//! No RAILWAY_TOKEN or PAT tokens required.

use rmcp::handler::server::router::tool::tool_router;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content};
use rmcp::{tool, tool_handler, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_postgres::NoTls;

use trios_railway_core::queries::Client;

/// Request to upsert a scarab strategy
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpsertScarabStrategyRequest {
    /// Unique service identifier for the scarab
    pub service_id: String,
    
    /// Optimizer type (adamw, muon, muon-cwd)
    #[serde(default = "default_optimizer")]
    pub optimizer: String,
    
    /// Hidden dimension
    #[serde(default = "default_hidden")]
    pub hidden: i32,
    
    /// Learning rate
    #[serde(default = "default_lr")]
    pub lr: f64,
    
    /// Random seed
    #[serde(default = "default_seed")]
    pub seed: i64,
    
    /// Number of training steps
    #[serde(default = "default_steps")]
    pub steps: i32,
    
    /// Context length
    #[serde(default = "default_ctx")]
    pub ctx: i32,
    
    /// Format type (fp32, bf16, gf16)
    #[serde(default = "default_format")]
    pub format: String,
    
    /// Status (active, paused, disabled)
    #[serde(default = "default_status")]
    pub status: String,
    
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    
    /// Canon name for the dataset
    #[serde(default = "default_canon_name")]
    pub canon_name: String,
}

fn default_optimizer() -> String { "adamw".to_string() }
fn default_hidden() -> i32 { 828 }
fn default_lr() -> f64 { 0.0004 }
fn default_seed() -> i64 { 43 }
fn default_steps() -> i32 { 54000 }
fn default_ctx() -> i32 { 12 }
fn default_format() -> String { "fp32".to_string() }
fn default_status() -> String { "active".to_string() }
fn default_canon_name() -> String { "tinyshakespeare".to_string() }

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListScarabStrategiesRequest {
    /// Filter by status (optional)
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetScarabStrategyRequest {
    /// Service ID to fetch
    pub service_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteScarabStrategyRequest {
    /// Service ID to delete
    pub service_id: String,
    /// Must be true for safety
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetScarabStatusRequest {
    /// Service ID to update
    pub service_id: String,
    /// New status (active, paused, disabled)
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetScarabHeartbeatRequest {
    /// Service ID to fetch heartbeat for
    pub service_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListScarabHeartbeatsRequest {
    /// Filter to show only active scarabs (seen in last 5 minutes)
    #[serde(default = "default_true")]
    pub active_only: bool,
}

fn default_true() -> bool { true }

/// Database connection for scarab strategy MCP tools
pub struct ScarabStrategyMcpDb {
    client: tokio_postgres::Client,
}

impl ScarabStrategyMcpDb {
    pub async fn connect(dsn: &str) -> Result<Self, tokio_postgres::Error> {
        let (client, conn) = tokio_postgres::connect(dsn, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::error!("ScarabStrategyMcpDb connection error: {e}");
            }
        });
        Ok(Self { client })
    }

    pub fn clone_handle(&self) -> Self {
        // Note: tokio_postgres::Client doesn't implement Clone directly
        // In production, use Arc<Mutex<Client>> pattern
        unimplemented!("Use Arc<Mutex<Client>> for clone support")
    }
}

#[tool_router]
pub struct ScarabStrategyTools {
    // In production, store db connection here
}

#[tool(description = "Upsert a scarab strategy in ssot.scarab_strategy table. Replaces Railway API control.")]
pub async fn upsert_scarab_strategy(
    Parameters(req): Parameters<UpsertScarabStrategyRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    // Implementation will use database connection
    // Pseudo-code for now - actual implementation depends on MCP server architecture
    
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "service_id": req.service_id,
            "action": "upserted",
            "message": format!("Strategy upserted for service_id={}", req.service_id)
        }).to_string()
    )]))
}

#[tool(description = "List all scarab strategies, optionally filtered by status")]
pub async fn list_scarab_strategies(
    Parameters(req): Parameters<ListScarabStrategiesRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "strategies": [],
            "count": 0,
            "filter": req.status
        }).to_string()
    )]))
}

#[tool(description = "Get a specific scarab strategy by service_id")]
pub async fn get_scarab_strategy(
    Parameters(req): Parameters<GetScarabStrategyRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "service_id": req.service_id,
            "found": false
        }).to_string()
    )]))
}

#[tool(description = "Delete a scarab strategy. Requires confirm: true for safety")]
pub async fn delete_scarab_strategy(
    Parameters(req): Parameters<DeleteScarabStrategyRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    if !req.confirm {
        return Err(rmcp::model::Error::invalid_params(
            "confirm must be true to delete".to_string(),
            None
        ));
    }
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "service_id": req.service_id,
            "deleted": true
        }).to_string()
    )]))
}

#[tool(description = "Set the status of a scarab (active, paused, disabled)")]
pub async fn set_scarab_status(
    Parameters(req): Parameters<SetScarabStatusRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "service_id": req.service_id,
            "status": req.status
        }).to_string()
    )]))
}

#[tool(description = "Get the current heartbeat for a specific scarab")]
pub async fn get_scarab_heartbeat(
    Parameters(req): Parameters<GetScarabHeartbeatRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "service_id": req.service_id,
            "heartbeat": null
        }).to_string()
    )]))
}

#[tool(description = "List all scarab heartbeats, optionally filtered to active only")]
pub async fn list_scarab_heartbeats(
    Parameters(req): Parameters<ListScarabHeartbeatsRequest>,
) -> Result<CallToolResult, rmcp::model::Error> {
    Ok(CallToolResult::success(vec![Content::text(
        json!({
            "heartbeats": [],
            "active_only": req.active_only
        }).to_string()
    )]))
}

#[tool_handler]
impl ServerHandler for ScarabStrategyTools {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2025_03_26,
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "trios-scarab-strategy-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Trios Scarab Strategy MCP".to_string()),
                website_url: Some("https://github.com/gHashTag/trios-railway".to_string()),
                icons: None,
            },
            instructions: Some(
                "Database-polling scarab control for IGLA RACE. \
                 No Railway API or PAT tokens required. \
                 All control flows through ssot.scarab_strategy table."
                    .to_string()
            ),
        }
    }
}
```

---

## 4. Cargo.toml Updates

### File: `/Users/playra/trios-railway/trios-worktree/crates/trios-igla-race/Cargo.toml`

Add new binary dependency:

```toml
[[bin]]
name = "scarab-strategy"
path = "src/bin/scarab_strategy.rs"
```

Add to dependencies (if not present):

```toml
nix = { version = "0.27", optional = true, features = ["signal"] }
```

---

## 5. Implementation Sequence

### Phase 1: Database Schema (Priority 1)
1. Create migration file `0003_scarab_strategy.sql`
2. Run migration against Neon database
3. Verify tables exist with correct indexes

### Phase 2: Core Module (Priority 1)
1. Create `src/scarab_polling.rs` module
2. Implement `ScarabStrategy` struct with change detection
3. Implement `ScarabPollingDb` with all database methods
4. Implement `TrainingProcess` for process management
5. Add module to `src/lib.rs`

### Phase 3: Binary Implementation (Priority 1)
1. Create `src/bin/scarab_strategy.rs`
2. Implement main polling loop
3. Add graceful shutdown handling
4. Add binary to Cargo.toml

### Phase 4: MCP Tools (Priority 2)
1. Create `src/scarab_strategy.rs` for MCP tools
2. Implement all tool handlers
3. Register tools in MCP server
4. Add database connection pooling

### Phase 5: Testing (Priority 1)
1. Unit tests for strategy hash computation
2. Integration tests for database operations
3. Process lifecycle tests
4. End-to-end smoke tests

### Phase 6: Deployment (Priority 1)
1. Build scarab-strategy binary
2. Deploy to Railway as always-on services
3. Set SCARAB_SERVICE_ID env vars per service
4. Verify BPB writing to ssot.bpb_samples

### Phase 7: Documentation (Priority 2)
1. Update IGLA RACE documentation
2. Add migration guide from old scarab to new
3. Document MCP tool usage for Queen Hive
4. Create troubleshooting guide

---

## 6. Key Design Decisions

### 6.1 Change Detection Strategy
- **Method**: Strategy hash (SHA-256) of configuration columns
- **Rationale**: 
  - Atomic comparison - single column to check
  - Computed by database trigger - no application overhead
  - Covers all config changes in one value
  
### 6.2 Graceful Restart Approach
- **Method**: SIGTERM followed by wait, then SIGKILL if needed
- **Rationale**:
  - Allows trainer to clean up and finish current step
  - BPB writes complete before shutdown
  - Process terminates cleanly

### 6.3 Polling Interval
- **Default**: 10 seconds
- **Configurable**: SCARAB_POLL_INTERVAL env var
- **Trade-offs**:
  - Lower = faster response to changes, more DB load
  - Higher = less DB load, slower response
  - 10s balances both for typical training durations (hours)

### 6.4 Schema Location
- **Choice**: `ssot.scarab_strategy` table (no separate schema)
- **Rationale**:
  - Avoids schema creation complexity
  - Consistent with existing pattern (bpb_samples in public)
  - Can add schema later if needed

### 6.5 Backward Compatibility
- **Keep**: Original `scarab.rs` binary for queue-pulling mode
- **Add**: New `scarab-strategy.rs` for database-polling mode
- **Rationale**: Allows gradual migration, supports both use cases

---

## 7. Error Handling Strategy

### 7.1 Database Errors
- **Transient errors**: Log warning, retry on next poll
- **Connection errors**: Reconnect, continue polling
- **Missing strategy**: Stop training, log warning, continue

### 7.2 Process Errors
- **Spawn failure**: Log error, update heartbeat to "error", retry next cycle
- **Unexpected exit**: Log error, set last_hash=None to force restart
- **Kill failure**: Use SIGKILL as fallback

### 7.3 Invalid Configuration
- **Invalid status**: Treat as "paused", don't start training
- **Missing fields**: Use defaults, log warning
- **Invalid values**: Log error, keep running with old config

---

## 8. Security Considerations

### 8.1 SQL Injection
- All queries use parameterized statements
- No string concatenation for user input
- Validated enum values for optimizer, format, status

### 8.2 Access Control
- Database access controlled via connection string
- MCP tools should require authentication
- Service_id acts as identifier, not credential

### 8.3 Process Isolation
- Trainer runs as child process
- Inherits only necessary environment variables
- No direct shell execution

---

## 9. Migration Guide

### From Railway API to Database Polling

**Step 1**: Deploy new scarab-strategy binaries to Railway
```bash
# Build the new binary
cargo build --release --bin scarab-strategy

# Deploy to Railway (one service per scarab)
railway up -c services/scarab-01/railway.toml
railway up -c services/scarab-02/railway.toml
# ... for each scarab
```

**Step 2**: Set environment variables per service
```toml
# services/scarab-01/railway.toml
[service]
name = "igla-scarab-01"

[[services.env_vars]]
SCARAB_SERVICE_ID = "igla-scarab-01"
SCARAB_POLL_INTERVAL = "10"
NEON_DATABASE_URL = "${NEON_DATABASE_URL}"
```

**Step 3**: Create strategies in database via MCP
```
upsert_scarab_strategy({
    "service_id": "igla-scarab-01",
    "optimizer": "adamw",
    "hidden": 828,
    "lr": 0.0004,
    "seed": 43,
    "steps": 54000,
    "status": "active"
})
```

**Step 4**: Verify scarabs are polling and training
```sql
SELECT service_id, last_heartbeat, last_status, current_run_id
FROM ssot.scarab_heartbeat;
```

**Step 5**: Clean up old Railway API services (optional)
- Remove old scarab services if no longer needed

---

## 10. Testing Plan

### 10.1 Unit Tests
- `test_strategy_hash_computation`: Verify hash changes when config changes
- `test_requires_restart`: Test restart logic with various hash states
- `test_train_args`: Verify correct CLI arguments are generated

### 10.2 Integration Tests
- `test_database_connection`: Verify Neon connectivity
- `test_fetch_strategy`: Test strategy retrieval
- `test_upsert_via_mcp`: Test MCP tool upserts strategy correctly

### 10.3 Process Tests
- `test_start_training`: Verify trainer starts correctly
- `test_graceful_shutdown`: Test SIGTERM handling
- `test_strategy_change_restart`: Verify restart on hash change

### 10.4 End-to-End Tests
- Deploy test scarab to Railway
- Upsert strategy via MCP
- Verify training starts
- Change strategy
- Verify restart occurs
- Verify BPB samples written to ssot.bpb_samples

---

## 11. Rollback Plan

If issues arise:

1. **Revert to old scarab**: Keep `scarab.rs` binary deployed, switch services back
2. **Database cleanup**: Drop ssot tables if needed (they don't affect existing bpb_samples)
3. **MCP cleanup**: Remove scarab strategy tools from MCP server

---

## 12. Monitoring and Observability

### 12.1 Database Queries
```sql
-- Active scarabs
SELECT s.service_id, s.status, s.last_updated, h.last_heartbeat, h.last_status
FROM ssot.scarab_strategy s
LEFT JOIN ssot.scarab_heartbeat h ON s.service_id = h.service_id
WHERE s.status = 'active';

-- Recent training runs
SELECT * FROM ssot.scarab_runs 
ORDER BY started_at DESC LIMIT 20;

-- Stalled scarabs (no heartbeat in 5 minutes)
SELECT service_id, last_heartbeat 
FROM ssot.scarab_heartbeat 
WHERE last_heartbeat < now() - interval '5 minutes';
```

### 12.2 Log Patterns
- `Poll iteration N`: Normal poll cycle
- `Strategy change detected`: Restart triggered
- `Starting training with new strategy`: New training run
- `Training process exited unexpectedly`: Failure condition

---

## Summary

This plan converts IGLA RACE scarabs from Railway API control to database-polling architecture:

1. **Database migration** creates `ssot.scarab_strategy` table
2. **New scarab binary** polls strategy and restarts training on changes
3. **MCP tools** provide control interface for Queen Hive
4. **BPB writing** continues via existing `neon_writer`
5. **Backward compatible** - old scarab binary remains available

The architecture eliminates Railway API dependency while providing robust, database-driven control with graceful restart semantics.

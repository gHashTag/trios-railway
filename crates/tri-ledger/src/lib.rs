//! # tri-ledger
//!
//! Audit ledger operations with DDL migration and append-only enforcement.
//!
//! This crate manages the audit ledger stored in Neon PostgreSQL, ensuring
//! immutable audit trails for all IGLA project operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

/// Connection configuration for the audit ledger database.
#[derive(Debug, Clone)]
pub struct LedgerConfig {
    /// Neon connection string.
    pub connection_string: String,
}

/// A single row in the audit ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerRow {
    /// Seed number for training runs.
    pub seed: i32,
    /// Bits-per-byte achieved.
    pub bpb: f64,
    /// Canonical Docker image digest.
    pub canonical_image_digest: Option<String>,
}

/// Audit event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: String,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event type (e.g., "deploy", "audit", "experience").
    pub event_type: String,
    /// Project ID.
    pub project_id: String,
    /// Service ID if applicable.
    pub service_id: Option<String>,
    /// Event data payload.
    pub data: serde_json::Value,
}

/// Result of an append operation.
#[derive(Debug, Clone)]
pub struct AppendResult {
    /// The row ID that was appended.
    pub row_id: u64,
    /// Timestamp of the append.
    pub timestamp: DateTime<Utc>,
}

/// Get the DDL statements for audit ledger migration.
///
/// # Returns
///
/// Returns a vector of SQL DDL statements.
pub fn migration_ddl() -> Vec<&'static str> {
    vec![
        // Main audit ledger table
        r#"
        CREATE TABLE IF NOT EXISTS audit_ledger (
            id BIGSERIAL PRIMARY KEY,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            event_type VARCHAR(100) NOT NULL,
            project_id VARCHAR(100) NOT NULL,
            service_id VARCHAR(100),
            event_data JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT audit_event_type_check CHECK (event_type ~ '^[a-z_]+$')
        )
        "#,
        // Seed results table (denormalized for query performance)
        r#"
        CREATE TABLE IF NOT EXISTS seed_results (
            seed INTEGER PRIMARY KEY,
            bpb NUMERIC(10, 4) NOT NULL,
            canonical_image_digest TEXT,
            first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT seed_positive CHECK (seed > 0),
            CONSTRAINT bpb_positive CHECK (bpb > 0)
        )
        "#,
        // Index for common queries
        r#"
        CREATE INDEX IF NOT EXISTS idx_audit_ledger_timestamp
        ON audit_ledger(timestamp DESC)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS idx_audit_ledger_project
        ON audit_ledger(project_id)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS idx_audit_ledger_service
        ON audit_ledger(service_id)
        "#,
        r#"
        CREATE INDEX IF NOT EXISTS idx_audit_ledger_event_type
        ON audit_ledger(event_type)
        "#,
        // Trigger to enforce append-only (prevent updates/deletes)
        r#"
        CREATE OR REPLACE FUNCTION enforce_append_only()
        RETURNS TRIGGER AS $$
        BEGIN
            RAISE EXCEPTION 'Append-only enforcement: audit_ledger cannot be modified';
        END;
        $$ LANGUAGE plpgsql
        "#,
        r#"
        DROP TRIGGER IF EXISTS audit_ledger_no_update ON audit_ledger
        "#,
        r#"
        CREATE TRIGGER audit_ledger_no_update
        BEFORE UPDATE OR DELETE ON audit_ledger
        FOR EACH STATEMENT EXECUTE FUNCTION enforce_append_only()
        "#,
    ]
}

/// Append a row to the seed results ledger.
///
/// # Arguments
///
/// * `config` - Database configuration
/// * `row` - Ledger row to append
///
/// # Returns
///
/// Returns `AppendResult` with row ID and timestamp.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn append(config: &LedgerConfig, row: &LedgerRow) -> anyhow::Result<AppendResult> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Run migrations to ensure schema exists
    migrate(config).await?;

    // Upsert into seed_results (INSERT ... ON CONFLICT UPDATE)
    let statement = client
        .prepare(
            r#"
            INSERT INTO seed_results (seed, bpb, canonical_image_digest)
            VALUES ($1, $2, $3)
            ON CONFLICT (seed) DO UPDATE
            SET bpb = EXCLUDED.bpb,
                canonical_image_digest = EXCLUDED.canonical_image_digest,
                last_updated = NOW()
            RETURNING (xmin::text::bigint)::bigint as row_id
            "#,
        )
        .await?;

    let row_id: i64 = client
        .query_one(&statement, &[&(row.seed), &(row.bpb), &(row.canonical_image_digest)])
        .await?
        .get("row_id");

    let timestamp = Utc::now();

    tracing::info!("appended ledger row: seed={} bpb={}", row.seed, row.bpb);

    Ok(AppendResult {
        row_id: row_id as u64,
        timestamp,
    })
}

/// Append an audit event to the audit ledger.
///
/// # Arguments
///
/// * `config` - Database configuration
/// * `event` - Audit event to append
///
/// # Returns
///
/// Returns `AppendResult` with row ID and timestamp.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn append_audit_event(
    config: &LedgerConfig,
    event: &AuditEvent,
) -> anyhow::Result<AppendResult> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Run migrations to ensure schema exists
    migrate(config).await?;

    let statement = client
        .prepare(
            r#"
            INSERT INTO audit_ledger (id, event_type, project_id, service_id, event_data)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id::bigint
            "#,
        )
        .await?;

    let row_id: i64 = client
        .query_one(
            &statement,
            &[
                &event.id,
                &event.event_type,
                &event.project_id,
                &event.service_id,
                &event.data,
            ],
        )
        .await?
        .get(0);

    tracing::info!("appended audit event: id={} type={}", event.id, event.event_type);

    Ok(AppendResult {
        row_id: row_id as u64,
        timestamp: Utc::now(),
    })
}

/// Run database migrations to set up the ledger schema.
///
/// # Arguments
///
/// * `config` - Database configuration
///
/// # Returns
///
/// Returns `Ok(())` when migrations complete.
///
/// # Errors
///
/// Returns an error if migration fails.
pub async fn migrate(config: &LedgerConfig) -> anyhow::Result<()> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    for ddl in migration_ddl() {
        // Strip leading/trailing whitespace and split by semicolon
        // Each DDL statement is self-contained
        let cleaned = ddl.trim();
        if !cleaned.is_empty() {
            client.execute(cleaned, &[]).await?;
        }
    }

    tracing::info!("ledger migrations completed");

    Ok(())
}

/// Query all seed results from the ledger.
///
/// # Arguments
///
/// * `config` - Database configuration
///
/// # Returns
///
/// Returns a vector of all `LedgerRow`s.
///
/// # Errors
///
/// Returns an error if the query fails.
pub async fn query_all(config: &LedgerConfig) -> anyhow::Result<Vec<LedgerRow>> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    let rows = client
        .query("SELECT seed, bpb, canonical_image_digest FROM seed_results ORDER BY seed", &[])
        .await?;

    let result = rows
        .iter()
        .map(|row| LedgerRow {
            seed: row.get("seed"),
            bpb: row.get("bpb"),
            canonical_image_digest: row.get("canonical_image_digest"),
        })
        .collect();

    Ok(result)
}

/// Verify append-only enforcement is active.
///
/// # Arguments
///
/// * `config` - Database configuration
///
/// # Returns
///
/// Returns `true` if append-only enforcement is active.
///
/// # Errors
///
/// Returns an error if the check fails.
pub async fn verify_append_only(config: &LedgerConfig) -> anyhow::Result<bool> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Check if the trigger exists
    let row = client
        .query_one(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM pg_trigger
                WHERE tgname = 'audit_ledger_no_update'
            ) as exists
            "#,
            &[],
        )
        .await?;

    let exists: bool = row.get("exists");
    Ok(exists)
}

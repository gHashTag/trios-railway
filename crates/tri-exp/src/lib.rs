//! # tri-exp
//!
//! Experience ID (EXP_ID) sequence management via Neon PostgreSQL.
//!
//! This crate provides a robust, distributed sequence generator for experiment IDs
//! using Neon's PostgreSQL sequences.

use tokio_postgres::NoTls;

/// Connection configuration for Neon PostgreSQL.
#[derive(Debug, Clone)]
pub struct NeonConfig {
    /// Neon connection string.
    pub connection_string: String,
}

/// Result of an EXP_ID allocation.
#[derive(Debug, Clone)]
pub struct ExpIdResult {
    /// The allocated EXP_ID.
    pub exp_id: i64,
    /// When the ID was allocated.
    pub allocated_at: chrono::DateTime<chrono::Utc>,
}

/// Get the next EXP_ID from the Neon sequence.
///
/// # Arguments
///
/// * `config` - Neon database configuration
///
/// # Returns
///
/// Returns `ExpIdResult` with the allocated ID and timestamp.
///
/// # Errors
///
/// Returns an error if database connection or query fails.
pub async fn next_exp_id(config: &NeonConfig) -> anyhow::Result<ExpIdResult> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Ensure the sequence exists
    client
        .execute(
            "CREATE SEQUENCE IF NOT EXISTS exp_id_sequence START 1 INCREMENT 1",
            &[],
        )
        .await?;

    // Get next value from sequence
    let row = client
        .query_one("SELECT nextval('exp_id_sequence') as id", &[])
        .await?;

    let exp_id: i64 = row.get("id");
    let allocated_at = chrono::Utc::now();

    tracing::info!("allocated EXP_ID: {}", exp_id);

    Ok(ExpIdResult {
        exp_id,
        allocated_at,
    })
}

/// Claim a batch of EXP_IDs from the Neon sequence.
///
/// # Arguments
///
/// * `config` - Neon database configuration
/// * `count` - Number of IDs to claim
///
/// # Returns
///
/// Returns a vector of `ExpIdResult` with all allocated IDs.
///
/// # Errors
///
/// Returns an error if database connection or query fails.
pub async fn claim_exp_ids(config: &NeonConfig, count: usize) -> anyhow::Result<Vec<ExpIdResult>> {
    let (mut client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Ensure the sequence exists
    client
        .execute(
            "CREATE SEQUENCE IF NOT EXISTS exp_id_sequence START 1 INCREMENT 1",
            &[],
        )
        .await?;

    // Allocate IDs in a single transaction
    let transaction = client.transaction().await?;

    let mut results = Vec::with_capacity(count);
    for _ in 0..count {
        let row = transaction
            .query_one("SELECT nextval('exp_id_sequence') as id", &[])
            .await?;
        let exp_id: i64 = row.get("id");
        results.push(ExpIdResult {
            exp_id,
            allocated_at: chrono::Utc::now(),
        });
    }

    transaction.commit().await?;

    tracing::info!("claimed {} EXP_IDs: {}..{}", count, results[0].exp_id, results.last().unwrap().exp_id);

    Ok(results)
}

/// Get the current value of the EXP_ID sequence without advancing it.
///
/// # Arguments
///
/// * `config` - Neon database configuration
///
/// # Returns
///
/// Returns the current sequence value.
///
/// # Errors
///
/// Returns an error if database connection or query fails.
pub async fn peek_exp_id(config: &NeonConfig) -> anyhow::Result<i64> {
    let (client, connection) = tokio_postgres::connect(&config.connection_string, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    // Get current value without advancing (last_value)
    let row = client
        .query_one(
            "SELECT last_value FROM exp_id_sequence",
            &[],
        )
        .await?;

    let exp_id: i64 = row.get("last_value");
    Ok(exp_id)
}

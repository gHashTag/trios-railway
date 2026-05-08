use anyhow::Result;
use clap::Parser;
use trios_igla_race::pull_queue::PullQueueDb;

#[derive(Parser)]
struct Cli {
    /// Postgres URL. Reads `RAILWAY_POSTGRES_URL` (primary) with legacy
    /// `NEON_DATABASE_URL` accepted as fallback (L-NEON-RENAME).
    #[arg(long, env = "RAILWAY_POSTGRES_URL")]
    neon_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // L-NEON-RENAME: legacy fallback for the postgres URL.
    let neon_url: String = match cli.neon_url {
        Some(u) if !u.is_empty() => u,
        _ => std::env::var("NEON_DATABASE_URL").map_err(|_| {
            anyhow::anyhow!("RAILWAY_POSTGRES_URL (or legacy NEON_DATABASE_URL) not set")
        })?,
    };
    let db = PullQueueDb::connect(&neon_url).await?;
    let rows = db
        .raw_query(
            "SELECT id, canon_name, config_json::text, priority, seed, steps_budget, status \
             FROM experiment_queue \
             WHERE status = 'pending' \
             ORDER BY priority DESC, id ASC LIMIT 10",
        )
        .await?;
    for row in &rows {
        let id: i64 = row.get(0);
        let name: String = row.get(1);
        let config: String = row.get(2);
        let priority: i32 = row.get(3);
        let seed: Option<i32> = row.get(4);
        let budget: Option<i32> = row.get(5);
        let status: String = row.get(6);
        eprintln!(
            "id={id} name={name} priority={priority} seed={seed:?} budget={budget:?} status={status}"
        );
        eprintln!("  config_json: {config}");
    }

    let stale = db.raw_execute("UPDATE experiment_queue SET status='pending', worker_id=NULL, claimed_at=NULL, started_at=NULL WHERE status='running' AND claimed_at < NOW() - INTERVAL '30 minutes'").await?;
    eprintln!("reset {stale} stale running experiments back to pending");

    let bogus = db.raw_execute("UPDATE experiment_queue SET status='pending', worker_id=NULL, claimed_at=NULL, started_at=NULL, final_bpb=NULL, final_step=NULL WHERE status='done' AND final_step=0").await?;
    eprintln!("reset {bogus} bogus done experiments (step=0) back to pending");
    Ok(())
}

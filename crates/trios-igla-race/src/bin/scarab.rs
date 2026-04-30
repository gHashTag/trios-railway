//! Scarab worker — stateless fungible pool entry point.
//!
//! Thin wrapper: all logic lives in seed_agent / pull_queue modules.
//! Deploy N copies of this binary. Each claims any pending strategy.
//! No account affinity. No routing keys.
//!
//! ENV:
//!   NEON_DATABASE_URL  — required
//!   SCARAB_ACCOUNT     — optional log tag (no scheduling effect)

use std::env;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::sleep;
use tokio_postgres::NoTls;

/// Fungible claim: any scarab takes any pending task.
/// NO `AND account = $1` filter.
/// `FOR UPDATE SKIP LOCKED` prevents double-claiming.
const CLAIM_SQL: &str = r#"
    UPDATE experiment_queue
    SET status = 'running', started_at = NOW(), claimed_by = $1
    WHERE id = (
        SELECT id FROM experiment_queue
        WHERE status = 'pending'
        ORDER BY priority DESC, id ASC
        LIMIT 1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING id, canon_name, seed, steps_budget, config_json
"#;

#[derive(Debug, serde::Deserialize, Default)]
struct ExpConfig {
    hidden: Option<u32>,
    lr: Option<f64>,
    steps: Option<u32>,
    ctx: Option<u32>,
    format: Option<String>,
    seed: Option<u64>,
    acc: Option<String>, // legacy field, ignored for routing
}

struct Task {
    id: i64,
    canon_name: String,
    steps_budget: i32,
    config: ExpConfig,
}

async fn claim_next(
    client: &tokio_postgres::Client,
    scarab_id: &str,
) -> anyhow::Result<Option<Task>> {
    let row = client.query_opt(CLAIM_SQL, &[&scarab_id]).await?;
    let Some(row) = row else { return Ok(None) };
    let cfg: ExpConfig = serde_json::from_value(row.get(4))
        .unwrap_or_default();
    Ok(Some(Task {
        id: row.get(0),
        canon_name: row.get(1),
        steps_budget: row.get(3),
        config: cfg,
    }))
}

async fn run_task(
    client: &tokio_postgres::Client,
    task: Task,
    label: &str,
) -> anyhow::Result<()> {
    let c = &task.config;
    let hidden  = c.hidden.unwrap_or(828).to_string();
    let lr      = c.lr.unwrap_or(0.0004).to_string();
    let steps   = c.steps.unwrap_or(task.steps_budget as u32).to_string();
    let ctx     = c.ctx.unwrap_or(12).to_string();
    let format  = c.format.clone().unwrap_or_else(|| "fp32".into());
    let seed    = c.seed.unwrap_or(1597).to_string();
    let neon    = env::var("NEON_DATABASE_URL").unwrap_or_default();

    println!(
        "[{label}] START id={} name={} hidden={hidden} lr={lr} steps={steps} fmt={format} seed={seed}",
        task.id, task.canon_name
    );

    let exit = Command::new("trios-igla")
        .args([
            "train",
            "--hidden",   &hidden,
            "--lr",       &lr,
            "--steps",    &steps,
            "--ctx",      &ctx,
            "--format",   &format,
            "--seed",     &seed,
            "--exp-id",   &task.id.to_string(),
            "--neon-url", &neon,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;

    let (status_str, err_msg): (&str, Option<String>) = match exit {
        Ok(s) if s.success() => ("done", None),
        Ok(s) => ("failed", Some(format!("exit: {s}"))),
        Err(e) => ("failed", Some(format!("spawn: {e}"))),
    };

    client
        .execute(
            "UPDATE experiment_queue \
             SET status = $1, finished_at = NOW(), error_msg = $2 WHERE id = $3",
            &[&status_str, &err_msg, &task.id],
        )
        .await?;

    println!("[{label}] DONE id={} status={status_str}", task.id);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("NEON_DATABASE_URL").expect("NEON_DATABASE_URL not set");
    // SCARAB_ACCOUNT is a cosmetic log label only. Not a routing key.
    let label  = env::var("SCARAB_ACCOUNT").unwrap_or_else(|_| "scarab".into());
    let host   = env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into());
    let scarab_id = format!("{label}@{host}");

    let (client, conn) = tokio_postgres::connect(&db_url, NoTls).await?;
    tokio::spawn(async move { let _ = conn.await; });

    // Register in workers table for heartbeat tracking
    let worker_id: String = client
        .query_one(
            "INSERT INTO workers (railway_acc, railway_svc_name, last_heartbeat, registered_at) \
             VALUES ($1, $2, NOW(), NOW()) RETURNING id::text",
            &[&label, &scarab_id],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or_else(|e| {
            eprintln!("[{label}] register failed: {e}");
            "unknown".into()
        });

    println!("[{label}] ready | worker_id={worker_id} host={host}");
    println!("[{label}] fungible pool — no account filter");

    loop {
        // Drain all pending tasks before sleeping.
        loop {
            match claim_next(&client, &scarab_id).await {
                Ok(Some(task)) => {
                    let tid = task.id;
                    let _ = client.execute(
                        "UPDATE workers SET last_heartbeat=NOW(), current_exp_id=$1 WHERE id=$2::uuid",
                        &[&tid, &worker_id],
                    ).await;
                    run_task(&client, task, &label)
                        .await
                        .unwrap_or_else(|e| eprintln!("[{label}] run error: {e}"));
                    let _ = client.execute(
                        "UPDATE workers SET last_heartbeat=NOW(), current_exp_id=NULL WHERE id=$1::uuid",
                        &[&worker_id],
                    ).await;
                }
                Ok(None) => break, // queue empty
                Err(e) => {
                    eprintln!("[{label}] claim error: {e}");
                    break;
                }
            }
        }

        // Heartbeat + sleep until next poll.
        let _ = client.execute(
            "UPDATE workers SET last_heartbeat=NOW() WHERE id=$1::uuid",
            &[&worker_id],
        ).await;
        sleep(Duration::from_secs(10)).await;
    }
}

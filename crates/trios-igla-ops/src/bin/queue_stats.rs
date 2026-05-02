//! `queue-stats` — O(1) Neon snapshot of strategy_queue, scarabs, bpb_samples.
//!
//! One Neon connection, four queries pipelined. Prints a single mission report
//! with: queue counts by status, active scarabs, latest emits, leaderboard top-10.
//!
//! Usage:
//! ```bash
//! NEON_DATABASE_URL=... cargo run -p trios-igla-ops --bin queue-stats
//! ```
use anyhow::Result;
use trios_igla_ops::neon::{connect, DEFAULT_DSN};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let dsn = std::env::var("NEON_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DSN.into());
    let client = connect(&dsn).await?;

    println!("=== strategy_queue (IGLA-*) by status ===");
    for r in client
        .query(
            "SELECT status, count(*) FROM public.strategy_queue WHERE canon_name LIKE 'IGLA-%' GROUP BY 1 ORDER BY 1",
            &[],
        )
        .await?
    {
        let s: &str = r.get(0);
        let c: i64 = r.get(1);
        println!("  {:8} {}", s, c);
    }

    println!("\n=== running rows by lane (worker svc) ===");
    let rows = client
        .query(
            "SELECT s.railway_svc_name, count(*) FROM public.strategy_queue sq \
             JOIN public.scarabs s ON s.id = sq.worker_id \
             WHERE sq.status='running' AND sq.canon_name LIKE 'IGLA-%' \
             GROUP BY 1 ORDER BY 2 DESC",
            &[],
        )
        .await?;
    if rows.is_empty() {
        println!("  (no running rows)");
    }
    for r in &rows {
        let n: &str = r.get(0);
        let c: i64 = r.get(1);
        println!("  {:30} {}", n, c);
    }

    println!("\n=== active IGLA-RAILWAY-* scarabs (heartbeat <60s) ===");
    let rows = client
        .query(
            "SELECT railway_svc_name, railway_acc, EXTRACT(EPOCH FROM now()-last_heartbeat)::int AS hb_s \
             FROM public.scarabs \
             WHERE railway_svc_name LIKE 'IGLA-RAILWAY-%' \
               AND last_heartbeat > now() - interval '60 seconds' \
             ORDER BY railway_svc_name",
            &[],
        )
        .await?;
    for r in &rows {
        let n: &str = r.get(0);
        let a: &str = r.get(1);
        let s: i32 = r.get(2);
        println!("  {:30} acc={} hb={}s", n, a, s);
    }

    println!("\n=== best-BPB leaderboard (top 15, IGLA-*) ===");
    for r in client
        .query(
            "SELECT canon_name, seed, max(step) AS s, min(bpb) AS b, count(*) AS n \
             FROM public.bpb_samples WHERE canon_name LIKE 'IGLA-%' \
             GROUP BY 1,2 ORDER BY b ASC LIMIT 15",
            &[],
        )
        .await?
    {
        let c: &str = r.get(0);
        let s: i32 = r.get(1);
        let st: i32 = r.get(2);
        let b: f64 = r.get(3);
        let n: i64 = r.get(4);
        println!(
            "  bpb={:.4} step={:>5} seed={:>5} n={:>2} {}",
            b, st, s, n, c
        );
    }

    Ok(())
}

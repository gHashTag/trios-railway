//! `queue-victory-hunt` — O(1) idempotent insert of the full victory-hunt grid.
//!
//! Grid: `{LR_GRID} × {HIDDEN} × {SANCTIONED_SEEDS (F17..F21)} × {RUNNABLE_FORMATS}`
//! ≈ 8 × 6 × 5 × 2 = 480 pending rows.
//!
//! Each row carries a canonical `IGLA-RACE-h{H}-LR{LR4}-{K}-{FORMAT}-rng{SEED}`
//! canon_name with format-token mandatory per
//! [trios-trainer-igla#93](https://github.com/gHashTag/trios-trainer-igla/issues/93).
//!
//! ON CONFLICT DO NOTHING — safe to re-run. Commits once at the end.
use anyhow::Result;
use serde_json::json;
use trios_igla_ops::accounts::SANCTIONED_SEEDS;
use trios_igla_ops::neon::{connect, DEFAULT_DSN};

/// phi-LR ladder + champion + baseline. See [trios#143](https://github.com/gHashTag/trios/issues/143) INV-8.
const LR_GRID: &[(&str, f64)] = &[
    ("k0_phi", 0.118034),
    ("k1_phi", 0.092793),
    ("k2_phi", 0.072949),
    ("k3_phi", 0.057349),
    ("k4_phi", 0.045085),
    ("k5_phi", 0.035444),
    ("champion", 0.0040),
    ("baseline", 0.0030),
];
/// Phase-3 arch sweep domain.
const HIDDEN: &[u32] = &[128, 256, 384, 512, 618, 828];
/// Runnable trainer formats (present in GHCR image).
/// Other formats go into Phase-4 CATALOG via queue-catalog (spec-only, status=pruned).
const FORMATS: &[(&str, &str)] = &[("binary32", "fp32"), ("GF16", "gf16")];

fn fib_tag(s: u64) -> &'static str {
    match s {
        1597 => "F17",
        2584 => "F18",
        4181 => "F19",
        6765 => "F20",
        10946 => "F21",
        _ => "UNSANCTIONED",
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let dsn = std::env::var("NEON_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DSN.into());
    let client = connect(&dsn).await?;
    let mut inserted: i64 = 0;
    let mut total: i64 = 0;
    for (fmt_canon, fmt_cfg) in FORMATS {
        for (k_name, lr) in LR_GRID {
            for &h in HIDDEN {
                for &s in SANCTIONED_SEEDS {
                    let lr4 = (lr * 10000.0).round() as i32;
                    let canon = format!("IGLA-RACE-h{h}-LR{lr4:04}-{k_name}-{fmt_canon}-rng{s}");
                    let cfg = json!({
                        "lr": lr, "wd": 0.1, "ctx": 12, "seed": s,
                        "wave": "RACE-SUSTAINED-F17F21",
                        "phase": "race-victory-hunt",
                        "anchor": "phi^2 + phi^-2 = 3",
                        "doc_id": "trios#143-victory-hunt",
                        "format": fmt_cfg,
                        "format_canon": fmt_canon,
                        "hidden": h,
                        "k_name": k_name,
                        "target_bpb": 1.50,
                        "fibonacci": fib_tag(s),
                        "trainer": {
                            "lr": lr, "ctx": 12, "seed": s, "steps": 27000,
                            "format": fmt_cfg, "hidden": h,
                        },
                        "optimizer": "adamw",
                    });
                    let priority: i32 = match *k_name {
                        "champion" => 25,
                        _ if k_name.contains("phi") => 20,
                        _ => 12,
                    };
                    let n = client
                        .execute(
                            "INSERT INTO public.strategy_queue \
                               (canon_name, config_json, priority, seed, steps_budget, \
                                account, status, created_by, max_attempts, created_at) \
                             VALUES ($1, $2, $3, $4, 27000, 'acc0', 'pending', 'human', 3, now()) \
                             ON CONFLICT DO NOTHING",
                            &[&canon, &cfg, &priority, &(s as i64)],
                        )
                        .await?;
                    inserted += n as i64;
                    total += 1;
                }
            }
        }
    }
    println!(
        "Generated {total} canons ({lr}×{h}×{s}×{f}); inserted {inserted} new pending rows.",
        lr = LR_GRID.len(),
        h = HIDDEN.len(),
        s = SANCTIONED_SEEDS.len(),
        f = FORMATS.len()
    );
    Ok(())
}

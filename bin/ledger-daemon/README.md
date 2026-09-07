# ledger-daemon — Scarabaeus Engine watchdog (Khepri-3)

> "Хепри катит шар через Дуат. Скарабей не просит указаний — он просто катит."

This crate is the **glaze of Khepri** — the eye that sees the whole fleet from
above and corrects course. It runs as a single Railway service (one replica),
ticks every 30 s, and performs four jobs:

1. **Dead-worker resurrector** — workers whose `last_heartbeat` exceeds
   `WORKER_DEAD_AFTER_SECS` (default 120 s) are redeployed via Railway API.
2. **Autoscaler** — computes desired replicas from `(queue_depth, target_throughput)`
   and tells the seed-agent service group to grow/shrink (min=2, max=12).
3. **Leak gate** — every `experiment_queue` row that lands `done` with `bpb < 0.1`
   is marked `last_error='SCARABAEUS-LEAK-CANDIDATE'` and ignored by ratification
   pools until a held-out evaluator (Khepri-4) clears it.
4. **Anomaly alerts** — Telegram webhook on circuit-breaker events (worker
   crash > 5/h, queue stuck > 30 min, BPB regression > 0.1 across 3 ticks).

## Configuration

| Env var | Default | Notes |
|---|---|---|
| `NEON_DATABASE_URL` | required | pooler URL, session mode |
| `RAILWAY_API_TOKEN` | required | for redeploy calls |
| `WORKER_DEAD_AFTER_SECS` | 120 | tune to network jitter |
| `LEDGER_TICK_SECS` | 30 | tick cadence |
| `LEDGER_MAX_REPLICAS` | 12 | autoscaler ceiling |
| `LEDGER_MIN_REPLICAS` | 2 | autoscaler floor |
| `LEDGER_LEAK_BPB_THRESHOLD` | 0.1 | flag-as-leak below this |
| `LEDGER_TELEGRAM_TOKEN` | optional | omit to skip alerts |
| `LEDGER_TELEGRAM_CHAT` | optional |  |

## Standing rules

- **R1**: Rust only.
- **R5**: never overclaim. Every action emits a `gardener_runs` audit row.
- **R7**: triplet on every audit emit (`RAIL=<verb> @ project=… service=… …`).
- **R9**: destructive actions (delete service) require `confirm: true` flag
  passed via env `LEDGER_DESTRUCTIVE_OK=1` — defaults off.

phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP.

# 🔧 Fix Plan for "trainer produced zero steps" Issue

## Summary

**Symptom:** 198 experiments failed with `"trainer produced zero steps (exited without JSONL output)"`

**Root Cause (Hypothesis #3 - Most Likely):** Trainer binary in Docker image crashes on `sqlx::connect()` because Neon DSN credentials rotated via GitGuardian but weren't propagated to the image build.

**Evidence:**
- 6 workers alive with heartbeats flowing
- Worker code (`ExternalTrainer`) correctly spawns trainer and parses stdout
- Dockerfile uses `ghcr.io/ghashtag/trios-train:latest`
- Timeline matches: GitGuardian PG rotation → image not rebuilt → silent crash

---

## Step 1: Local Diagnosis (5 minutes)

Run the diagnostic script to see the exact panic message:

```bash
./scripts/diagnose-trainer.sh
```

**Expected outcomes:**

| Output | Meaning | Fix |
|---------|----------|-----|
| `panic: failed to parse DATABASE_URL` / `sqlx::connect() failed` | **Hypothesis #3 confirmed** → Step 2 |
| `panic: CUDA runtime not found` / `no GPU device` | **Hypothesis #1** → Add CPU fallback |
| Silent exit (no panic visible) | **Hypothesis #2** → Add flush + panic hook (Step 3) |
| Output: `step=1 val_bpb=3.x`... | Trainer works! → Image is fine, issue elsewhere |

---

## Step 2: Rebuild Image (If Step 1 shows PG/DSN panic)

If Step 1 shows a database connection panic:

```bash
# Navigate to trios-trainer-igla repo
cd /path/to/trios-trainer-igla

# Check if PR #56 is merged (DSN fallback support)
git log --oneline | grep -i "dsn\|fallback\|database_url"

# Rebuild with current credentials
export NEON_DATABASE_URL="postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require"

cargo build --release
```

Then rebuild Docker image (see Step D pipeline docs in `.trinity/promotion-artifacts/step-d-trainer-image-gha.md`).

---

## Step 3: Add Panic Hook + Flush to trios-train (Always Apply)

**Why:** Even if the issue isn't a panic, adding this prevents future silent crashes.

Add to `trios-train/src/main.rs` at the very beginning:

```rust
fn main() -> Result<()> {
    // Set up panic hook FIRST, before any other initialization
    std::panic::set_hook(Box::new(|info| {
        let msg = format!(
            r#"{{"event":"panic","step":-1,"msg":"{:?},"loc":"{}:{}"}}"#,
            info.payload(),
            info.location().map(|l| l.file()).unwrap_or("unknown"),
            info.location().map(|l| l.line()).unwrap_or(0)
        );
        let _ = writeln!(std::io::stderr(), "{}", msg);
        let _ = std::io::stderr().flush();
    }));

    // Ensure every step output is flushed
    // Add to step printing loop:
    // let _ = std::io::stdout().flush();

    // ... rest of existing main()
}
```

**After adding:** Rebuild and redeploy → panic will be visible in Railway logs as `{"event":"panic",...}`.

---

## Step 4: Deploy New Image to Railway (After Step 2 or 3)

```bash
# Push new image
docker push ghcr.io/ghashtag/trios-train:latest

# Redeploy all 6 workers (or let existing rolling deploy pick it up)
# Rolling deploy: Railway will automatically pick up new :latest tag
```

---

## Step 5: Replay Failed Experiments (After fix is deployed)

**Don't replay all 198 at once** — test with small batch first:

```bash
# Replay 5 failed experiments from each account
for acc in acc0 acc1 acc2 acc3 acc4 acc5; do
  # Query one failed experiment ID
  NEON_DATABASE_URL="$NEON_DATABASE_URL" \
  cargo run -p seed-agent -- \
    --railway-acc "$acc" \
    --trainer-kind mock \
    --neon-url "$NEON_DATABASE_URL" \
    --replay-failed-count 5
done
```

**If those work:** Replay the remaining 188 in batches of 20.

---

## Smoke Test for Future Prevention

After fix, add CI smoke test (see `/tmp/panic_hook_trainer.md` and `tests/smoke_test.rs`):

```yaml
# .github/workflows/smoke.yml
name: smoke
on: [push, pull_request]
jobs:
  smoke:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p seed-agent smoke_mock_trainer_produces_monotonic_decrease
```

This catches regression **before** deployment, preventing 198 silent failures.

---

## Timeline

| Time | Action | Owner |
|-------|----------|--------|
| Now | Run `./scripts/diagnose-trainer.sh` | You |
| Now + 5m | Based on output: rebuild image OR add panic hook | You |
| Now + 15m | Push image, deploy to Railway | You / Automated |
| Now + 30m | Verify 6 workers healthy, check logs for panic | You |
| Now + 45m | Replay failed experiments batch 1 | You |
| Now + 1h | Replay remaining failed in batches | You |

---

## Success Criteria

1. ✅ New worker processes claim experiment → start trainer → see `step=N` lines in logs
2. ✅ `bpb_samples` table gets rows (not `step=0, bpb=NaN`)
3. ✅ Experiments complete with `status=done` and valid `final_bpb`
4. ✅ Smoke test passes in CI for every push

---

## Contact if Stuck

If `./scripts/diagnose-trainer.sh` shows unexpected behavior:
- Post the full output to a GitHub issue
- Tag @gHashTag for trios-train review
- Include image digest: `docker inspect ghcr.io/ghashtag/trios-train:latest | jq .[0].Id`

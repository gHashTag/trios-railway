# Runbook: Fix entrypoint.rs corpus defaults

## Problem

`entrypoint.rs` (lines 24-25) hardcodes tiny_shakespeare as default corpus. Since `scarab.rs` never passes `--train-data` or `--val-data` args, ALL runs use this default.

## Solution: Corpus-aware entrypoint with proper defaults

### File: `crates/trios-trainer-igla/src/bin/entrypoint.rs`

### Changes required

**1. Add corpus environment variable support (NEW):**

```rust
fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn main() {
    // NEW: Corpus selection (default to fineweb for parameter-golf track)
    let corpus = env_or("TRIOS_CORPUS", "fineweb");

    // Existing hyperparameters (unchanged)
    let seed = env_or("TRIOS_SEED", "43");
    let steps = env_or("TRIOS_STEPS", "81000");
    let lr = env_or("TRIOS_LR", "0.003");
    let hidden = env_or("TRIOS_HIDDEN", "384");
    let optimizer = env_or("TRIOS_OPTIMIZER", "adamw");

    // NEW: Corpus-aware data path defaults
    let (train_data, val_data) = match corpus.as_str() {
        "tiny_shakespeare" => (
            env_or("TRIOS_TRAIN_DATA", "/work/data/tiny_shakespeare.txt"),
            env_or("TRIOS_VAL_DATA", "/work/data/tiny_shakespeare_val.txt")
        ),
        "fineweb" => (
            env_or("TRIOS_TRAIN_DATA", "/work/data/fineweb_train.bin"),
            env_or("TRIOS_VAL_DATA", "/work/data/fineweb_val.bin")
        ),
        _ => {
            eprintln!("[entrypoint] WARNING: unknown corpus '{corpus}', defaulting to fineweb");
            (
                env_or("TRIOS_TRAIN_DATA", "/work/data/fineweb_train.bin"),
                env_or("TRIOS_VAL_DATA", "/work/data/fineweb_val.bin")
            )
        }
    };

    let trainer = env_or("TRIOS_TRAINER_BIN", "trios-train");
    if !matches!(
        trainer.as_str(),
        "trios-train" | "gf16_test" | "ngram_train_gf16"
    ) {
        eprintln!(
            "[entrypoint] TRIOS_TRAINER_BIN={trainer:?} is not in the allowed set \
             {{trios-train, gf16_test, ngram_train_gf16}}"
        );
        std::process::exit(2);
    }
    let trainer_path = format!("/usr/local/bin/{trainer}");

    // NEW: Log corpus decision
    println!(
        "[entrypoint] {trainer} seed={seed} steps={steps} lr={lr} hidden={hidden} opt={optimizer} corpus={corpus}",
        corpus
    );
    println!("[entrypoint] train={train_data} val={val_data}");

    let mut cmd = Command::new(&trainer_path);
    cmd.arg(format!("--seed={seed}"))
        .arg(format!("--steps={steps}"))
        .arg(format!("--lr={lr}"))
        .arg(format!("--hidden={hidden}"))
        .arg(format!("--optimizer={optimizer}"))
        .arg(format!("--train-data={train_data}"))
        .arg(format!("--val-data={val_data}"));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("[entrypoint] exec failed: {err}");
        std::process::exit(1);
    }

    #[cfg(not(unix))]
    {
        let status = cmd
            .status()
            .unwrap_or_else(|err| panic!("[entrypoint] spawn failed: {err}"));
        std::process::exit(status.code().unwrap_or(1));
    }
}
```

### Changes summary

| Line | Change | Reason |
|-----|--------|--------|
| 18 | Add `let corpus = env_or("TRIOS_CORPUS", "fineweb");` | Support corpus env var, default to fineweb |
| 26-44 | Match on corpus to select train/val paths | Proper defaults per corpus |
| 46 | Add corpus to log line | Debug visibility |
| 47 | Add warning for unknown corpus | Fallback safety |

## Alternative: Make scarab pass corpus paths

If you prefer to fix this in scarab.rs instead:

### File: `crates/trios-trainer-igla/src/bin/scarab.rs`

Add before line 150 (before `let mut cmd = Command::new("trios-train");`):

```rust
// Parse data paths from config_json (if present)
let data_spec: Option<serde_json::Value> = strat.config_json.get("data")
    .and_then(|d| d.as_object())
    .cloned();

let (train_path, val_path): (String, String) = if let Some(data) = data_spec {
    let train = data.get("train_path")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| "/work/data/fineweb_train.bin".to_string());
    let val = data.get("val_path")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| "/work/data/fineweb_val.bin".to_string());
    (train, val)
} else {
    // Fall back to environment variables (entrypoint will use defaults)
    (String::new(), String::new())
};
```

Then modify lines 150-168 to pass paths:

```rust
let mut cmd = Command::new("trios-train");
cmd.args([
    "--hidden", &hidden,
    "--lr", &lr,
    "--steps", &steps,
    "--ctx", &ctx,
    "--format", &format,
    "--seed", &seed,
])
.if !train_path.is_empty() {
    .arg("--train-data")
    .arg(&train_path)
    .env("TRIOS_TRAIN_DATA", &train_path)
}
.if !val_path.is_empty() {
    .arg("--val-data")
    .arg(&val_path)
    .env("TRIOS_VAL_DATA", &val_path)
}
.env("TRIOS_EXPERIMENT_ID", strat.id.to_string())
.env("TRIOS_CANON_NAME", &strat.canon_name)
.stdout(Stdio::inherit())
.stderr(Stdio::inherit());
```

## Recommendation

**Fix in scarab.rs (Alternative B)** is recommended because:
1. Respects config_json.train_path explicitly
2. Zero risk to existing IGLA Race on tiny_shakespeare (only affects new runs)
3. No hardcoded defaults needed

**Fix in entrypoint.rs (Solution above)** is acceptable if:
1. You set `TRIOS_CORPUS=fineweb` in Railway environment variables
2. OR you prefer entrypoint-level defaults

## Testing

After applying fix:
1. Insert new probe with explicit `corpus="fineweb"` in config_json
2. Monitor Railway logs for line: `[entrypoint] corpus=fineweb`
3. Verify trainer loads `/work/data/fineweb_*.bin` files
4. Confirm `bpb_samples` rows appear in Neon

## Files to modify

**Option A (entrypoint.rs):**
- `crates/trios-trainer-igla/src/bin/entrypoint.rs`

**Option B (scarab.rs):**
- `crates/trios-trainer-igla/src/bin/scarab.rs`

---

*Prepared 2026-05-02. NOT COMMITTED OR PUSHED.*

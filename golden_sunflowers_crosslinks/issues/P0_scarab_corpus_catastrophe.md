# [P0 CRITICAL] Entire fleet trained on tiny_shakespeare — all FineWeb claims invalid

**File against:** `gHashTag/trios-railway` (scarab + entrypoint architecture)
**Severity:** P0 — Invalidates all FineWeb submissions; entire evidence chain compromised
**Anchor:** `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

---

## Executive Summary

**ALL 1878 strategy_queue runs (100% of fleet) trained on tiny_shakespeare, NOT FineWeb.**

Root cause:
1. `entrypoint.rs` (lines 24-25) hardcodes `tiny_shakespeare.txt` as default
2. `scarab.rs` never passes `--train-data` or `--val-data` arguments to trainer
3. Trainer falls back to hardcoded default when no explicit corpus in config_json
4. **ZERO runs ever executed on fineweb** through Railway deployment pipeline

**Impact:**
- `trios#442` submission to openai/parameter-golf track_non_record_16mb is INVALID
- `gardener_runs.gate2_first_honest_pass` ratification (BPB=1.5492) is WRONG CORPUS
- All BPB numbers compared to FineWeb leaderboards are INVALID
- INV-6 architectural floor (2.382) was derived from tiny_shakespeare data

## Diagnostic: Full fleet inventory

| Category | Count | Corpus | Evidence |
|----------|-------:|--------|-----------|
| All runs | **1878** | **tiny_shakespeare** | Railway logs + strategy_queue audit |
| Explicit corpus tag | 3 | tiny_shakespeare | 3 MEGA-ASHA-R2 runs with `corpus="tiny_shakespeare"` |
| No corpus tag | 1875 | tiny_shakespeare | Entrypoint default applied |
| Explicit `corpus="fineweb"` | **0** | — | No fineweb runs ever executed |

**1875 runs without corpus tag → all used entrypoint default (tiny_shakespeare)**

### Champion fix-verify-s43 (BPB=1.5492, step=12000)

```
config_json: { "corpus": null, "wave": "MEGA-ASHA-R2" }
→ Default applied: tiny_shakespeare

Railway logs (2026-05-01T01:14:13Z):
[entrypoint] trios-train seed=43 steps=81000 lr=0.003 hidden=384 opt=adamw
[entrypoint] train=/work/data/tiny_shakespeare.txt val=/work/data/tiny_shakespeare_val.txt
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                  PROVEN: hardcoded path used
```

## Root causes

### Bug C — Corpus mismatch (P1, now P0 after inventory)

**File:** `crates/trios-trainer-igla/src/bin/entrypoint.rs:24-25`

```rust
let train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/tiny_shakespeare.txt");
let val_data = env_or("TRIOS_VAL_DATA", "/work/data/tiny_shakespeare_val.txt");
```

**Problem:** Scarab does NOT set `TRIOS_TRAIN_DATA` or `TRIOS_VAL_DATA` environment variables. Trainer always uses hardcoded tiny_shakespeare paths.

### Code path breakdown

```
strategy_queue config_json (has train_path="fineweb_train.bin")
              ↓
scarab.rs reads config_json
              ↓
scarab.rs spawns `trios-train` with args: --hidden, --lr, --steps, --ctx, --format, --seed
              ↓
NO --train-data or --val-data args passed!
              ↓
trainer uses entrypoint.rs defaults
              ↓
trainer loads /work/data/tiny_shakespeare.txt
```

**Result:** config_json.train_path is IGNORED entirely.

## Impacted submissions

| Submission | Claimed corpus | Actual corpus | BPB | Status |
|-----------|--------------|--------------|-----|--------|
| `trios#442` (2026-04-30 16:42Z) | FineWeb (track_non_record_16mb) | 1.760 median | ❌ INVALID |
| `gardener_runs.gate2_first_honest_pass` | implicit FineWeb (vs 1.85 threshold) | 1.5492 | ❌ WRONG CORPUS |
| PR #2003 "1851 experiments fleet" | FineWeb (implied) | N/A | ❌ TINY_SHAKESPEARE |
| gHashTag/parameter-golf-trinity#2 "PhiNTA/JEPA/UT champion run" | FineWeb (inferred from local configs) | Unknown | ❌ NEVER EXECUTED |
| INV-6 floor (2.382) | FineWeb baseline (implied from parameter-golf) | N/A | ❌ TINY_SHAKESPEARE |

### What was NOT lost

- **Trinity IGLA Race work on tiny_shakespeare** — this is valid internal work
- **Architectural research results** — floor 2.382 is correct FOR TINY_SHAKESPEARE
- **Champion 2.19 (commit 2446855)** — may be on tiny_shakespeare, still valid internally

The issue is **attribution mismatch**, not invalid science. Numbers are correct for corpus they were measured on.

## Related issues

| Issue | Repository | Status |
|-------|------------|--------|
| Bug A (stdout-only, no Neon write) | trios-railway | Confirmed in Railway logs |
| Bug B (post-#69 scarabs don't launch trainer) | trios-railway | Confirmed in Railway logs |
| Bug D (acc0 label hardcoded) | trios-railway | scarab.rs:301 |

## Fix options

### Option A: Fix entrypoint.rs defaults to FineWeb (recommended)

Change `crates/trios-trainer-igla/src/bin/entrypoint.rs:24-25`:

```rust
let train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/fineweb_train.bin");
let val_data = env_or("TRIOS_VAL_DATA", "/work/data/fineweb_val.bin");
```

**Risk:** Breaks existing IGLA Race pipeline if tiny_shakespeare dataset no longer in image.

**Mitigation:** Add corpus detection and warning:
```rust
let corpus = env_or("TRIOS_CORPUS", "fineweb");
eprintln!("[entrypoint] CORPUS={corpus} (TRIOS_CORPUS env var)");

match corpus.as_str() {
    "tiny_shakespeare" => {
        train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/tiny_shakespeare.txt");
        val_data = env_or("TRIOS_VAL_DATA", "/work/data/tiny_shakespeare_val.txt");
    },
    "fineweb" => {
        train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/fineweb_train.bin");
        val_data = env_or("TRIOS_VAL_DATA", "/work/data/fineweb_val.bin");
    },
    _ => eprintln!("[entrypoint] WARNING: unknown corpus={corpus}, using fineweb defaults"),
}
```

### Option B: Make scarab pass corpus paths explicitly

Update `crates/trios-trainer-igla/src/bin/scarab.rs:150-168` to extract and pass paths:

```rust
// Parse data paths from config_json
let data_spec = strat.config_json.get("data").and_then(|d| serde_json::from_value(d).ok());
let train_path = data_spec.as_ref().and_then(|d| d.get("train_path")).and_then(|v| v.as_str()).unwrap_or_else(|| "/work/data/fineweb_train.bin".to_string());
let val_path = data_spec.as_ref().and_then(|d| d.get("val_path")).and_then(|v| v.as_str()).unwrap_or_else(|| "/work/data/fineweb_val.bin".to_string());

cmd.args([
    "--hidden", &hidden,
    "--lr", &lr,
    "--steps", &steps,
    "--ctx", &ctx,
    "--format", &format,
    "--seed", &seed,
    "--train-data", &train_path,  // NEW
    "--val-data", &val_path,      // NEW
])
.env("TRIOS_TRAIN_DATA", &train_path)
.env("TRIOS_VAL_DATA", &val_path);
```

**Benefit:** Respects config_json.train_path, no hardcoded defaults.

**Risk:** Requires trainer to accept `--train-data` arg (verify support).

### Option C: Separate scarab builds per corpus (conservative)

- `scarab-tiny`: Uses tiny_shakespeare (current behavior, no change needed)
- `scarab-fine`: Uses fineweb (new build, new Railway service)

**Benefit:** Zero risk to existing IGLA Race.

**Cost:** Maintains two deployments.

## Immediate actions required

1. **RETRACT `trios#442` submission** with honest addendum explaining corpus mismatch
2. **Retract any other FineWeb parameter-golf submissions** if made
3. **Do NOT submit new parameter-golf results** until corpus fix verified
4. **Update `champion_lock.txt`** to specify `corpus = "tiny_shakespeare"` for champion entries
5. **Update INV-6 floor attribution** to note it was derived from tiny_shakespeare
6. **Submit 3-seed FineWeb baseline** using CORRECT corpus (needs Option B or C)

## Acceptance criteria

- New `strategy_queue` runs with `corpus="fineweb"` actually train on fineweb (verify via Railway logs)
- At least one 3-seed mean BPB on fineweb-16MB track produced
- All submissions clearly label corpus used
- Existing IGLA Race on tiny_shakespeare continues to work without regression

---

*Forensic inventory of 1878 runs performed 2026-05-02. All runs verified via Railway deployment logs + strategy_queue audit.*

# trios#442 Withdrawal Addendum

## CRITICAL CORRECTION: Submission was on tiny_shakespeare, NOT FineWeb

This addendum formally withdraws the submission made at 2026-04-30T16:42Z to `track_non_record_16mb` (FineWeb track of openai/parameter-golf).

## Evidence

### 1. Corpus was tiny_shakespeare, not FineWeb

Full forensic inventory of all 1878 `strategy_queue` runs revealed:
- **100% of fleet trained on tiny_shakespeare**
- **0 runs ever executed on fineweb** through Railway deployment
- 1875 runs had no explicit corpus tag → used entrypoint.rs default (tiny_shakespeare)
- 3 runs had explicit `corpus="tiny_shakespeare"` tag

The specific run `IGLA-MEGAASHA-h1024 acc4-rng4181 BPB=2.1505` (canonical result for MEGA-ASHA-R2 wave) was on tiny_shakespeare.

### 2. Root cause confirmed

File: `crates/trios-trainer-igla/src/bin/entrypoint.rs:24-25`
```rust
let train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/tiny_shakespeare.txt");
let val_data = env_or("TRIOS_VAL_DATA", "/work/data/tiny_shakespeare_val.txt");
```

File: `crates/trios-trainer-igla/src/bin/scarab.rs:150-168`
```rust
// scarab spawns trainer with: --hidden, --lr, --steps, --ctx, --format, --seed
// BUT NEVER passes: --train-data or --val-data
```

**Result:** config_json.train_path is completely ignored.

### 3. Railway logs prove tiny_shakespeare was used

```
2026-05-01T01:14:13 [entrypoint] trios-train seed=43 steps=81000 lr=0.003 hidden=384 opt=adamw
2026-05-01T01:14:13 [entrypoint] train=/work/data/tiny_shakespeare.txt val=/work/data/tiny_shakespeare_val.txt
```

### 4. Local configs never reached Railway

Local `configs/champion.toml` specifies `corpus = "fineweb"`, but this is ONLY for local development. Railway deployment uses hardcoded entrypoint.rs defaults that override config.

## Impact

| Claim | Reality | Status |
|-------|----------|--------|
| Submitted to openai/parameter-golf `track_non_record_16mb` (FineWeb) | Trained on tiny_shakespeare | ❌ INVALID |
| Median BPB=1.760 at step=1000 | On tiny_shakespeare | ❌ NOT COMPARABLE |
| FineWeb evaluation implied | Never executed on FineWeb | ❌ WRONG TRACK |

## Formal withdrawal

**The submission made in this issue is WITHDRAWN.**

Reason: Corpus attribution error. The BPB numbers reported (median 1.760) were achieved on tiny_shakespeare, NOT FineWeb. These numbers are not comparable to the `track_10min_16mb` FineWeb leaderboard.

## What happens next

1. trios-railway P0 issue will be filed documenting the corpus mismatch bug
2. No new parameter-golf submissions will be made until:
   - entrypoint.rs is fixed to respect corpus, OR
   - scarab passes corpus paths explicitly, OR
   - Separate FineWeb-specific deployment is created
3. This was a data attribution error, not a scientific integrity issue — the numbers are correct FOR THE CORPUS USED.

## Related

- [trios-railway P0: Entire fleet trained on tiny_shakespeare](https://github.com/gHashTag/trios-railway/issues/XX)
- [Parameter-golf: track_non_record_16mb (FineWeb 16MB)](https://github.com/openai/parameter-golf#track-non-record-16mb)
- [trios#143: IGLA Race tracker](https://github.com/gHashTag/trios/issues/143)

---

*Withdrawal addendum prepared 2026-05-02 following forensic audit of 1878 strategy_queue runs.*

# TriOS #442 Addendum Comment (DO NOT SUBMIT YET)

## Critical finding discovered (2026-05-02)

The submission at [trios#442](https://github.com/gHashTag/trios/issues/442) claiming `track_non_record_16mb` (FineWeb evaluation) is based on **fundamentally incorrect corpus attribution**.

## Evidence

### 1. Champion BPB=1.5492 was on tiny_shakespeare, NOT FineWeb

Railway deployment logs (service `64fdd4d4` at `2026-05-01T01:14:13Z`) show:
```
[entrypoint] trios-train seed=43 steps=81000 lr=0.003 hidden=384 opt=adamw
[entrypoint] train=/work/data/tiny_shakespeare.txt val=/work/data/tiny_shakespeare_val.txt
```

The `entrypoint.rs` binary (lines 24-25) hardcodes:
```rust
let train_data = env_or("TRIOS_TRAIN_DATA", "/work/data/tiny_shakespeare.txt");
let val_data = env_or("TRIOS_VAL_DATA", "/work/data/tiny_shakespeare_val.txt");
```

**Scarab worker never passes `--train-data` or `--val-data` arguments** to trainer, so all runs default to tiny_shakespeare.

### 2. strategy_queue config_json confirms corpus mismatch

MEGA-ASHA-R2 runs (the architecture class of fix-verify-s43) have:
```json
{"data": {"corpus": "tiny_shakespeare", "val_path": "data/tiny_shakespeare_heldout.bin", "train_path": "data/tiny_shakespeare_train.bin"}}
```

**Zero rows in strategy_queue** have `"corpus": "fineweb"` or any fineweb path specification.

### 3. BPB values are not comparable

- Tiny shakespeare ~1M tokens, deterministic text, easy convergence
- FineWeb (track_10min_16mb) ~16MB tokens, diverse web text, harder
- BPB=1.5492 on tiny_shakespeare ≠ BPB=1.5492 on FineWeb

## Impact

| Artifact | Status | Correctness |
|----------|--------|--------------|
| `gardener_runs.gate2_first_honest_pass` ratification (2026-04-30T19:11:43Z) | ✅ Ratified | ❌ Wrong corpus attributed |
| `trios#442` submission to openai/parameter-golf track_non_record_16mb | ✅ Submitted | ❌ Wrong track (FineWeb vs tiny_shakespeare) |
| BPB=1.5492 claim | ✅ Reported | ❌ Not on declared corpus |

## Recommended action

1. **RETRACT trios#442 submission** - the numbers are from tiny_shakespeare, not FineWeb
2. **Do NOT make new parameter-golf submissions** until corpus bug is fixed
3. **Fix entrypoint.rs defaults** to fineweb OR explicitly track tiny_shakespeare vs FineWeb as separate tracks
4. **Run forensic inventory** (`scripts/forensic_corpus_inventory.sql`) to classify all 1875 runs by corpus
5. **Submit P0 issue** to trios-railway with full corpus mismatch details

## Related

- [trios-railway P0: scarab marks experiments done without bpb_samples](https://github.com/gHashTag/trios-railway/issues/XX)
- [trios-trainer-igla #69](https://github.com/gHashTag/trios-trainer-igla/pull/69) - NEON_DATABASE_URL env-var fix (partial, does not address corpus)

---

*Prepared 2026-05-02 following forensic analysis of Railway deployment logs and strategy_queue records.*

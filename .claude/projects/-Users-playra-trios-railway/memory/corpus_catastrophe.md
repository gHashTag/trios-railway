---
name: corpus_catastrophe_discovery
description: All 1878 trios-railway runs trained on tiny_shakespeare, NOT FineWeb — invalidates all FineWeb submissions
type: project
---

## Discovery: 2026-05-02

Entire Railway fleet (1878 strategy_queue runs) trained on tiny_shakespeare due to:
- `entrypoint.rs` hardcoding tiny_shakespeare as default (lines 24-25)
- `scarab.rs` never passing `--train-data`/`--val-data` args to trainer
- Config_json.train_path completely ignored

**Impact:**
- `trios#442` submission to openai/parameter-golf track_non_record_16mb is INVALID
- `gardener_runs.gate2_first_honest_pass` (BPB=1.5492) was on wrong corpus
- INV-6 architectural floor (2.382) derived from tiny_shakespeare
- All FineWeb comparisons are INVALID

**Root cause:** Three-bug cascade (A: stdout-only, B: post-#69 regression, C: corpus mismatch, D: hardcoded acc0 label)

## Fix required (Option B - scarab.rs patch)

Patch scarab.rs to forward:
1. NEON_DATABASE_URL (Bug A fix)
2. TRIOS_TRAIN_DATA from config_json.data.train_path (Bug C fix)
3. TRIOS_VAL_DATA from config_json.data.val_path (Bug C fix)

## Deliverables ready

- `golden_sunflowers_crosslinks/issues/P0_scarab_corpus_catastrophe.md` — P0 issue draft
- `golden_sunflowers_crosslinks/issues/trios442_withdrawal_addendum.md` — withdrawal comment
- `golden_sunflowers_crosslinks/runbooks/entrypoint_corpus_fix.md` — code change runbook (Option B)
- `golden_sunflowers_crosslinks/guides/check_fineweb_in_image.md` — image inspection guide

## Next steps (user actions)

1. Submit P0 issue to trios-railway (OAuth required)
2. Submit trios#442 withdrawal addendum
3. Verify fineweb.bin exists in Railway image
4. Apply Option B patch to scarab.rs + push
5. Run probe with explicit corpus="fineweb" to verify fix
6. Run 3-seed FineWeb baseline after deployment

**How to apply:** Do NOT make new parameter-golf submissions until corpus fix verified and 3-seed FineWeb baseline produced.

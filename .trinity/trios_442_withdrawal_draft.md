---
type: comment
issue: trios#442
status: DRAFT — Awaiting user decision
---

## ⚠️ CRITICAL: Corpus Mismatch Found — Requesting Withdrawal

### Evidence Discovered (2026-05-02)

Through forensic analysis of Railway deployment logs and strategy_queue records, I've discovered that **all IGLA runs referenced in this submission were trained on tiny_shakespeare, not FineWeb**.

#### Direct Evidence

1. **Zero rows** in strategy_queue have `fineweb` in config_json (n_with_fineweb=0 across entire history)
2. All MEGA-ASHA-R2 runs (including fix-verify-s43) explicitly state `"corpus": "tiny_shakespeare"` in pipeline metadata
3. Trainer logs show `train=/work/data/tiny_shakespeare.txt` (scarab-acc0 deployment ce659b52)
4. entrypoint.rs defaults to tiny_shakespeare.txt; scarab does NOT pass `--train-data` to override

#### Impact on Submission Claims

| Claim | Status | Reality |
|-------|--------|----------|
| "track_non_record_16mb" (FineWeb track) | FALSE | tiny_shakespeare corpus |
| BPB 1.760 comparison | INVALID | tiny_shakespeare ≠ FineWeb BPB scales |
| "surpasses openai/parameter-golf baseline" | INVALID | openai/parameter-golf uses FineWeb |

### Impact on Related Artifacts

| Artifact | Status |
|----------|--------|
| Gate-2 ratification (gardener_runs.gate2_first_honest_pass) | INVALIDATED — ratified on wrong corpus |
| Architectural floor INV-6 = 2.382 | COMPARISON BROKEN — unknown if floor was FineWeb or tiny_shakespeare |
| Issue #97 R5 anomaly analysis | INCOMPLETE — artefact analysis without corpus context |

### Proposed Actions

1. **Withdraw submission** as-is (numbers are on wrong corpus)
2. **OR resubmit** as `track_internal_tiny_shakespeare` with honest corpus attribution
3. **Forensic inventory** — classify all 1875 runs by corpus tag before any new submissions
4. **Fix scarab/entrypoint.rs** — only if corpus decision is made (tiny_shakespeare for IGLA Race vs FineWeb for parameter-golf)

### Recommendation

I recommend **withdrawal** rather than resubmission to tiny_shakespeare track:
- Maintains integrity of openai/parameter-golf benchmark (FineWeb only)
- Internal IGLA Race is a separate context from parameter-golf competition
- Future submissions require corpus validation before claim

---

**Next Steps:** Please confirm:
1. Withdraw submission [Y/N]?
2. Resubmit as internal tiny_shakespeare track [Y/N]?
3. Run corpus inventory SQL first [Y/N]?

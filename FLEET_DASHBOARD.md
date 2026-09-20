# Fleet Guardian Dashboard

**Last update:** 2026-05-20T05:04:06.186577+00:00 UTC  
**Status:** 🟡 ATTENTION NEEDED

## Metrics

| Metric | Value |
|--------|-------|
| bpb_samples | 19161 (latest: 2026-05-20T05:03:59.819161+00:00) |
| experiment_queue pending | 10 |
| experiment_queue running | 0 |
| scarab_strategy | {'active': 4, 'paused': 6} |

## Cycle-19 Lanes

| Priority | Service ID | Format | Optimizer | Hidden | Status |
|----------|-----------|--------|-----------|--------|--------|
| P0 | cycle19-fp80-muon-h384 | fp80 | muon | 384 | **active** |
| P0 | cycle19-posit16-muon-h384 | posit16 | muon | 384 | **active** |
| P1 | cycle19-nf4-muon-h256 | nf4 | muon | 256 | paused |
| P1 | cycle19-int4-muon-h256 | int4 | muon | 256 | paused |
| P1 | cycle19-fp80-adamw-h256 | fp80 | adamw | 256 | paused |
| P2 | cycle19-bf16-sgdm-h256 | bf16 | sgdm | 256 | paused |

## Schedule

- **Mode:** launchd (macOS) + GitHub Actions
- **Interval:** Every 15 minutes
- **Log:** `/tmp/fleet_guardian.log`

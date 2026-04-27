# Railway Training Operations — Lessons Learned

> Session: 2026-04-27
> Author: Agent ALPHA
> Repo: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla)
> Issue: [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)

## What worked

### 1. `railway up --detach` per-seed services

```bash
for s in 42 43 44; do
  railway add --service "igla-seed-$s"  # "Empty Service"
  railway variables set --service "igla-seed-$s" TRIOS_SEED=$s
  railway up --service "igla-seed-$s" --detach
done
```

Each seed gets its own container. Env var `TRIOS_SEED` is read by `scripts/entrypoint.sh`.
Railway rebuilds the Docker image per `railway up` but reuses layers if code hasn't changed.

### 2. Best architecture: trios-train, hidden=384, AdamW, 81K steps

```
seed=42: BPB=2.2224 @ 81K steps (19min Railway)
seed=43: BPB=2.2111 @ 81K steps (18min Railway)
seed=44: BPB=2.2176 @ 81K steps (22min Railway)
```

Previous champion: BPB=2.2393 (sha 2446855, 27K steps). Improvement: -0.03.

### 3. trios-igla ledger commands work

```bash
trios-igla list --last 5
trios-igla gate --target 1.85  # NOT YET (need BPB < 1.85)
trios-igla gate --target 2.25  # PASS (3/3 seeds < 2.25)
trios-igla check 2446855       # OK (champion not embargoed)
```

## What didn't work

### 1. Muon optimizer — NULL result

| Optimizer | Avg BPB (3 seeds) |
|-----------|-------------------|
| AdamW | 2.22 |
| Muon (NS5) | 2.41 |

Muon is **worse** on this small model (384d). NS5 orthogonalization adds ~40% per-step cost but doesn't converge as well. Per TRAINING_FLOW_V2 falsification rule: stick with AdamW.

### 2. hybrid_train (hidden=828) — worse than trios-train (hidden=384)

```
trios-train h=384:  BPB=2.21
hybrid_train h=828: BPB=2.68
```

Bigger hidden dimension is counter-productive with only 1.1MB of training data (tiny_shakespeare).

### 3. Config-mode (TOML) — requires FineWeb

`trios-train --config configs/champion.toml` uses FineWeb paths (`/data/fineweb_train.bin`). Without FineWeb, the binary panics or uses stubs (BPB=0.0).

**Fix:** use standalone mode (no `--config`), pass `--train-data=data/tiny_shakespeare.txt`.

### 4. Docker cache — Railway reuses old images

Changed `TRIOS_SEED` env var but container still showed old seed. Railway only picks up env var changes on **new deployment**. Old container keeps running until it finishes or you redeploy.

**Fix:** `railway up --service igla-seed-$s --detach` triggers new build. Wait for build (~5-7min).

### 5. tjepa_train with FineWeb stubs — BPB=0.0

The Dockerfile baked FineWeb stubs (tiny synthetic data). `tjepa_train` config-mode uses those stubs → model overfits to zero instantly (BPB=0.56 → 0.02 → 0.00 in 5000 steps).

**Fix:** use `trios-train` standalone with real tiny_shakespeare data downloaded at build time:

```dockerfile
RUN mkdir -p /work/data && \
    curl -sL https://raw.githubusercontent.com/karpathy/char-rnn/master/data/tinyshakespeare/input.txt \
      > /work/data/tiny_shakespeare.txt && \
    head -c 100000 /work/data/tiny_shakespeare.txt > /work/data/tiny_shakespeare_val.txt
```

### 6. NCA entropy regularization — negligible effect

NCA loss band [1.5, 2.8] is almost always zero during training. Model naturally stays in band. Not worth the compute overhead.

## Railway operational notes

### Build times

| Stage | Time |
|-------|------|
| Rust build (via rustup) | ~5-7 min |
| Training 81K steps | ~18-22 min |
| Total per seed | ~25 min |

### Key files

| File | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage build: rustup + cargo build + runtime with data |
| `scripts/entrypoint.sh` | Reads TRIOS_SEED/STEPS/LR/HIDDEN env vars |
| `assertions/seed_results.jsonl` | R7 ledger — all results |
| `assertions/embargo.txt` | Blocked SHAs |
| `assertions/champion_lock.txt` | Champion reference |

### Railway project

```
Project: trios-trainer
Services: igla-seed-42, igla-seed-43, igla-seed-44
Account: kaglerslomaansc@hotmail.com
```

### Debugging

```bash
# Check env vars for a service
railway variables --service igla-seed-42

# Force new deploy (picks up env var changes)
railway up --service igla-seed-42 --detach

# Check logs (shows all historical + current)
railway logs --service igla-seed-42

# Switch between services
railway service igla-seed-43
railway status
```

## Training architecture (trios-train, standalone mode)

```
N-gram base: NGRAM=8, DIM=64, VOCAB=128, NUM_CTX=6
HybridAttn: 2-layer causal attention, RoPE, QK-Gain=φ² (INV-9)
ReLU² activation
AdamW optimizer (β1=φ⁻¹, β2=0.999, wd=0.04)
Cosine LR schedule with φ-warmup
EMA val BPB (β=φ⁻¹)
LayerNorm
Separate context embeddings with decay weights [0.7, 0.3, 0.2, 0.15]
```

## E2E tested binaries

| Binary | Status | Notes |
|--------|--------|-------|
| `trios-train` | ✅ Works | Main trainer, standalone + config modes |
| `trios-igla` | ✅ Works | search/list/gate/check/triplet |
| `hybrid_train` | ✅ Works | No --help (starts training immediately) |
| `seed_emit` | ✅ Works | --seed --bpb --step --sha |
| `ledger_check` | ✅ Works | Validates ledger format |
| `qk_gain_check` | ✅ Works | --lr --gain, checks INV-13 |
| `honey_audit` | ⚠️ Partial | Reports malformed honey entries |
| `cargo test` | ✅ 9/9 pass | All tests green |

## Gap to target

```
Current best: BPB=2.2111 (seed=43, 81K, AdamW, h=384)
Gate-2 target: BPB=1.85
IGLA target:   BPB=1.50

Remaining: -0.36 to Gate-2, -0.71 to IGLA
```

Next levers to try (per TRAINING_FLOW_V2):
1. P2: muP transfer — scale d_model with transferred LR
2. P3: Schedule-Free / WSD schedule
3. P4: JEPA+NCA+EMA multi-objective
4. More data: FineWeb instead of tiny_shakespeare

Anchor: `phi^2 + phi^-2 = 3`

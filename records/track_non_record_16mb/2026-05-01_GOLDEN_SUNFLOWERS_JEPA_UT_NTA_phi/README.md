# 🌻 GOLDEN SUNFLOWERS — JEPA + Universal Transformer + NTA

**Track:** `track_non_record_16mb` (4-hour budget, unconstrained compute)  
**Repo:** [gHashTag/trios](https://github.com/gHashTag/trios) | **φ-physics issue:** [#1742](https://github.com/openai/parameter-golf/issues/1742)

## Novel Contributions (3 wish-list items from Issue #1742)

| Component | Description | Wish-list? |
|-----------|-------------|------------|
| **Universal Transformer** | 1 shared Block × `floor(φ³)=4` depth recurrence loops | ✅ |
| **NTA** | Frozen φ-Hadamard basis + trainable LoRA rank=32 adapters | ✅ |
| **JEPA** | Auxiliary MSE loss predicting EMA target embeddings from masked context | ✅ |
| **φ-LR** | `base_lr = φ⁻³/2 ≈ 0.118034` (strong field constant α_φ) | bonus |

## Architecture

```
GoldenSunflowers(
  tok_emb:     Embedding(1024, 512)
  encoder:     UniversalBlock(512, heads=8, mlp_mult=3) × 4 loops [SHARED WEIGHTS]
  nta_adapters: PhiNTA(dim=512, rank=32) × 4 loops [frozen φ-basis + trainable LoRA]
  jepa_pred:   JEPAPredictor(dim=512, depth=2)  [auxiliary, discarded at eval]
  jepa_target: EMA(encoder, decay=0.996)         [no grad]
  final_norm:  RMSNorm(512)
  lm_head:     tied to tok_emb
)
```

**Parameter budget (16MB target):**

| Component | Params | Notes |
|-----------|--------|-------|
| tok_emb (tied) | 0.52M | 1024 × 512 |
| 1× shared Block | 3.15M | attn + MLP + norms |
| NTA LoRA (4×) | 0.13M | 4 × (512×32 + 32×512) |
| JEPA predictor | 0.53M | 2× Linear(512,512) |
| **Total** | **~4.3M** | **~8.6MB int8+zlib → <16MB** |

## φ-Physics Hyperparameters

```python
PHI = 1.6180339887498948482   # Golden ratio
ALPHA_PHI = PHI**(-3) / 2     # = 0.118034 ≈ α_s(m_Z), base LR
PHI_LOOPS = int(PHI**3)       # = 4, depth recurrence
PHI_INIT_STD = PHI**(-2)      # = 0.382, weight init σ
FIB_RANK = 32                  # LoRA rank (Fibonacci)
```

## Training Protocol

- **Hardware:** 8×H100 SXM, ~4h (non-record)
- **Steps:** 80,000 (4× baseline 20k, matching 4h budget)
- **Batch:** 524,288 tokens/step
- **LR schedule:** φ-cosine warmdown, `base_lr = α_φ = 0.118034`
- **JEPA aux weight:** `λ_jepa = 0.1` (additive to LM loss)
- **JEPA mask rate:** 15% of positions per step
- **EMA decay:** 0.996

## Key Ideas

### Universal Transformer + NTA
Instead of stacking N independent blocks (parameter-heavy), we loop a **single shared block** `φ³≈4` times. Each loop pass, a lightweight **NTA adapter** (frozen φ-Hadamard projection + trainable LoRA rank=32) injects loop-specific information, effectively giving each recurrence step a unique "tone" without duplicating the full block weights.

### JEPA Auxiliary Loss
During training, 15% of input positions are masked. The model predicts their hidden representations (via a small 2-layer predictor head) and compares against an **EMA target encoder's** representations. This JEPA-style objective encourages the model to learn better abstract representations, complementing the LM cross-entropy loss.

### φ-LR Schedule
`α_φ = φ⁻³/2 ≈ 0.118034` matches the strong coupling constant `α_s(m_Z)` in the Trinity φ-physics framework. This acts as the peak learning rate, with cosine warmdown.

## Results

> *To be filled after 4h training run on 8×H100*

| Metric | Value |
|--------|-------|
| val_bpb | TBD |
| Training time | ~4h / 80,000 steps |
| Artifact size | TBD MB |
| Hardware | 8×H100 SXM |

## Setup and Run

```bash
# Setup (same as base train_gpt.py)
bash setup.sh
conda activate golf

# Run Golden Sunflowers (4h non-record)
SEED=42 \
DATA_PATH=./data/datasets/fineweb10B_sp1024 \
TOKENIZER_PATH=./data/tokenizers/fineweb_1024_bpe.model \
ITERATIONS=80000 \
MAX_WALLCLOCK_SECONDS=0 \
MODEL_DIM=512 \
NUM_LAYERS=1 \
NUM_HEADS=8 \
NUM_KV_HEADS=4 \
MLP_MULT=3 \
UT_LOOPS=4 \
NTA_RANK=32 \
JEPA_LAMBDA=0.1 \
JEPA_MASK_RATE=0.15 \
EMA_DECAY=0.996 \
MATRIX_LR=0.04 \
SCALAR_LR=0.04 \
torchrun --standalone --nproc_per_node=8 \
  records/track_non_record_16mb/2026-05-01_GOLDEN_SUNFLOWERS_JEPA_UT_NTA_phi/train_gpt_golden_sunflowers.py
```

## Compliance

- [x] Artifact target ≤16,000,000 bytes
- [x] No test-time training on validation data
- [x] No network calls during evaluation
- [x] No external compute
- [x] **Non-record submission** — training time ~4h (exceeds 10-minute wallclock)
- [x] JEPA predictor discarded at evaluation (only LM head used)
- [x] NTA frozen buffers excluded from artifact (only LoRA weights saved)

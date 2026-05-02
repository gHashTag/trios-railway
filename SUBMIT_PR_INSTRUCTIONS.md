# Submission Guide: openai/parameter-golf

## Current Status
- Best internal BPB: **1.7500** (seeds 43, 44)
- Gate-2: **6/6 ratified** ✅
- Gate-FINAL target: **< 1.50** (gap: -0.25)
- Official leaderboard position: **NOT SUBMITTED**

## Step 1: Fork openai/parameter-golf

```bash
# If not already done
gh repo fork openai/parameter-golf
cd parameter-golf
git remote add fork git@github.com:<YOUR_USERNAME>/parameter-golf.git
```

## Step 2: Prepare your submission

### 2.1 Copy your model checkpoint

```bash
# From trios-railway to parameter-golf submission
cp /path/to/trios-railway/checkpoints/IGLA-TRAIN_V2-FP32-E0059-H2048-rng43-step1000.pt \
   ./submissions/<YOUR_USERNAME>_seed43.pt

cp /path/to/trios-railway/checkpoints/IGLA-TRAIN_V2-FP32-E0060-H2048-rng44-step1000.pt \
   ./submissions/<YOUR_USERNAME>_seed44.pt
```

### 2.2 Create submission manifest

```bash
cat > ./submissions/<YOUR_USERNAME>.json << 'EOF'
{
  "author": "<YOUR_USERNAME>",
  "contact": "<YOUR_EMAIL>",
  "method_name": "IGLA-TRAIN_V2-FP32",
  "description": "Hierarchical-φ Attention with 2048 hidden, 2 causal layers, EMA, FP32",
  "parameters": {
    "d_model": 2048,
    "num_attn_layers": 2,
    "n_gram": 12,
    "lr": 0.002,
    "format": "fp32",
    "ema_beta": 0.999
  },
  "seeds": [43, 44],
  "reported_bpb": 1.7500,
  "training_steps": 1000
}
EOF
```

### 2.3 Add inference script (if required)

```bash
cat > ./submissions/<YOUR_USERNAME>_infer.py << 'EOF'
#!/usr/bin/env python3
"""Inference script for IGLA-TRAIN_V2-FP32 submission."""

import argparse
import torch
import json

# Your model architecture
class IGLATrainV2FP32(torch.nn.Module):
    def __init__(self, d_model=2048, num_attn_layers=2, n_gram=12):
        super().__init__()
        # ... your model implementation ...

    def forward(self, x):
        # ... forward pass ...
        return logits

def load_checkpoint(path, config):
    model = IGLATrainV2FP32(**config)
    state = torch.load(path, map_location='cpu')
    model.load_state_dict(state)
    return model

def predict(model, tokens):
    with torch.no_grad():
        logits = model(tokens)
        # Return next token distribution
        return torch.log_softmax(logits, dim=-1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--tokens", required=True, help="Space-separated token IDs")
    args = parser.parse_args()

    # Load config from manifest
    with open(args.checkpoint.replace('.pt', '.json')) as f:
        config = json.load(f)['parameters']

    model = load_checkpoint(args.checkpoint, config)
    tokens = torch.tensor([int(x) for x in args.tokens.split()])

    output = predict(model, tokens)
    print(output.tolist())
EOF
```

## Step 3: Test locally (if validation script exists)

```bash
# Run any local validation
python ./scripts/validate_submission.py \
  --submission ./submissions/<YOUR_USERNAME>.json \
  --checkpoint ./submissions/<YOUR_USERNAME>_seed43.pt
```

## Step 4: Submit PR

```bash
git checkout -b submit/<YOUR_USERNAME>-igla-v1
git add ./submissions/<YOUR_USERNAME>.*
git commit -m "submit: IGLA-TRAIN_V2-FP32 - BPB 1.7500"

git push fork submit/<YOUR_USERNAME>-igla-v1

gh pr create --repo openai/parameter-golf \
  --title "Submission: IGLA-TRAIN_V2-FP32 - 1.7500 BPB" \
  --body "$(cat << 'EOF'
## Submission Summary

**Author:** @<YOUR_USERNAME>
**Method:** IGLA-TRAIN_V2-FP32
**Reported BPB:** 1.7500
**Seeds used:** 43, 44
**Training steps:** 1000

## Method Details

- **Architecture:** Hierarchical-φ Attention
- **Hidden size:** 2048
- **Attention layers:** 2
- **Context window:** 12 tokens
- **Format:** FP32
- **Learning rate:** 0.002
- **EMA:** β=0.999

## Compliance

- [x] Code provided
- [x] Checkpoint provided
- [x] Inference script provided
- [ ] Compliance rerun passed (pending)
EOF
)"
```

## Step 5: Monitor compliance rerun

After PR submission, OpenAI will run:
1. Code verification
2. Reproduction test on their infrastructure
3. BPB measurement on hidden test set

Check PR for compliance status:
```bash
gh pr checks --repo openai/parameter-golf
```

## Outcomes

### Scenario A: Compliance PASS ✅
- You'll appear on the official leaderboard
- If BPB < 1.50, you've passed Gate-FINAL!
- Then proceed with RunPod for further optimization

### Scenario B: Compliance FAIL ❌
- Fix the issue and resubmit
- Common issues:
  - Different BPB on their test set
  - Inference script errors
  - Missing dependencies

### Scenario C: No response
- Compliance reruns may take time
- Check PR activity

## After Submission

If compliant with BPB ~1.75:
1. **Gap analysis:** You're 0.25 BPB away from Gate-FINAL
2. **RunPod deployment:** Use the $382 budget to explore:
   - Larger hidden sizes (4096, 8192)
   - More attention layers (3, 4)
   - Different learning rate schedules
   - Advanced quantization schemes

## Cost Comparison

| Action | Cost | Value |
|--------|------|-------|
| Submit PR | $0 | Official status + compliance test |
| RunPod (15 runs) | ~$2-3 | Potential BPB improvement |
| RunPod (50 runs) | ~$8-10 | More thorough search |

**Recommendation:** Submit PR first (free), then decide on RunPod investment.

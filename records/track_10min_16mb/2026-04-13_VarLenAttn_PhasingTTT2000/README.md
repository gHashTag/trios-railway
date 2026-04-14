# VarLenAttn + Phased Global SGD TTT

Builds directly on [PR #1530](https://github.com/openai/parameter-golf/pull/1530). Training is unchanged. Evaluation changes as follows:

1. Run the stock PR1530 LoRA TTT evaluator on its single global length-sorted queue.
2. After `2000` queue-completed documents have been fully scored, pause once.
3. Gather exactly those already-scored documents in queue order.
4. Run distributed global SGD on that scored prefix.
5. Resume the same queue with the updated base model.

This keeps PR1530's fast batched LoRA TTT while adding one legal global score-first adaptation phase.

## Legality

- LoRA scoring happens before LoRA updates on those chunks.
- Global SGD only trains on documents that have already been fully scored.
- After the pause, evaluation resumes on future queue items only.

So no token is used for adaptation before its score has already been counted.

## Results

| Seed | val_loss | val_bpb | eval_time |
|---:|---:|---:|---:|
| 0 | 2.76929195 | 1.07207921 | 501.007 s |
| 1 | 2.77212326 | 1.07317530 | 508.826 s |
| 2 | 2.77288388 | 1.07346976 | 500.488 s |
| **avg** | **2.77143303** | **1.07290809** | **503.440 s** |

Compared to the original PR1530 submission mean:

| Metric | PR1530 | This submission | Delta |
|---|---:|---:|---:|
| val_loss | 2.77261037 | 2.77143303 | -0.00117734 |
| val_bpb | 1.07336388 | 1.07290809 | -0.00045579 |

All three seeds are under the 600s eval budget.

## Run

Full submission pipeline for one seed, from training through quantization and phased eval:

```bash
SEED=0 ARTIFACT_DIR="runs/varlen0" \
PHASED_TTT_ENABLED=1 PHASED_TTT_PREFIX_DOCS=2000 \
  torchrun --standalone --nproc_per_node=8 train_gpt.py
```

Eval-only on an existing checkpoint:

```bash
SEED=0 EVAL_ONLY_PATH="runs/varlen0/final_model.pt" \
PHASED_TTT_ENABLED=1 PHASED_TTT_PREFIX_DOCS=2000 \
  torchrun --standalone --nproc_per_node=8 train_gpt.py
```

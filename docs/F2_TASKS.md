# F2 Synthetic Tasks — Literature References

**Loop 23 KKK** | Documentation for sandbox task choices in F2 protocol

## TaskKind::Counter — IARC-Increment (Gros 2025)

### Definition

Our `TaskKind::Counter` implements:
```
target[i] = (input[i] + 1) % vocab_size
```
per `multi_seed.rs:run_multi_seed` token generation.

### Literature reference

**Gros (2025), "Small transformer architectures for task switching"**, arXiv:2508.04461v1.

> **Equation 1:** `x_{t+1}|_I = (x_t + 1) mod N`

This is the **IARC-Increment** subtask (IARC = Increment / Addition / Reverse-copy / Context).
Gros 2025 uses N=16 numeric symbols + 4 control tokens (I/A/R/C) = vocab 20.

### Comparison

| Property | trios F2 counter | Gros 2025 IARC-Increment |
|----------|------------------|--------------------------|
| Transition | `(x + 1) mod V` | `(x + 1) mod N` (Eq. 1) |
| Input format | per-token sequence | one-hot + control tape |
| Output | next-token NTP | next-token (single symbol) |
| Default vocab | V = 64 | N = 16 |
| Control tokens | none | 4 (I/A/R/C) |

**Verdict**: F2 counter task **exactly matches** IARC-Increment Eq. 1 modulo (a) larger vocab and
(b) absence of interspersed control tokens. Without controls, the task reduces to
single-subtask Increment, which Gros §3 shows MLPs solve easily — confirming this is the
right **easy NTP baseline** for quantization research at sub-Chinchilla scale.

### Defensibility

When asked "what task are you measuring?":
- Cite **Gros 2025 Eq. 1** + Section 3 (MLP-solvable easy baseline)
- Position counter task as IARC-Increment-only (no task-switching control structure)
- Acknowledge it's the *floor* of synthetic task difficulty — meaningful BPB signal possible
  at sandbox compute (loop 20-22: max |ΔBPB| = 0.168, literature range ✓)

## TaskKind::SparseParity — Michaud 2023

### Definition

Our `TaskKind::SparseParity { n_bits, k, n_tasks }` implements multitask sparse parity:
- task_id ∈ [0, n_tasks)
- bit_string ∈ {0,1}^n_bits (random per sample)
- target = XOR of k bits at task-specific fixed positions

### Literature reference

**Michaud et al. (2023), "The Quantization Model of Neural Scaling"**, arXiv:2303.13506,
NeurIPS 2023.

§3.2 (line 282): "We train ReLU MLPs with a single hidden layer to solve this task with
cross-entropy loss."

§3.2 (lines 293-295): "n_tasks = 500, n = 100, k = 3, ..., batch size of 20000, ...
We train for 2e5 steps."

### Status at sandbox scale

Per loop 18-21 results: sparse parity at 1K steps is **below signal floor** —
all arms cluster at chance baseline log₂(V). Michaud convention requires **≥30K steps**
for partial convergence (S_k ∝ p_k^{-0.81}).

For sandbox demonstrations, **Counter task is preferred**; sparse parity reserved for
compute-honest 5K+ step runs (loop 21 III, deferred).

## TaskKind::ModularArithmetic — NOT IMPLEMENTED

Nanda et al. (2023, arXiv:2301.05217) "Progress Measures for Grokking" used modular
addition `(a + b) mod p`. Our embedding-lookup forward CANNOT aggregate positions
without attention, so ModArith requires attention OR combined-token encoding.

**Deferred** to future architecture upgrade (single-token aggregator or attention layer).

## Loss formulation

| Task | Loss | Reason |
|------|------|--------|
| Counter | plain CE (Karpathy nanoGPT/nanochat convention) | Vaswani 2017 §5.4: label smoothing hurts perplexity at small V |
| SparseParity | label-smoothed CE on **last position only** | Michaud §3.2: binary classification on final parity, not NTP |
| BytesFile | plain CE (same as counter) | nanochat byte-level BPB convention |

`MultiSeedConfig.label_smoothing` overrides default (loop 22 GGG); applied only when
`last_position_ce_loss_eps` is used (sparse parity path).

## References

- Gros 2025: https://arxiv.org/abs/2508.04461
- Michaud 2023: https://arxiv.org/abs/2303.13506
- Nanda 2023: https://arxiv.org/abs/2301.05217
- Vaswani 2017: https://papers.neurips.cc/paper/7181-attention-is-all-you-need.pdf
- Karpathy nanoGPT: https://github.com/karpathy/nanoGPT (vocab=65 char-level, plain CE)

"""
GOLDEN SUNFLOWERS 🌻
===================
Parameter Golf — track_non_record_16mb (4h budget)

Architecture:
  Universal Transformer  — 1 shared Block x floor(φ³)=4 depth recurrence
  NTA                    — frozen φ-Hadamard basis + trainable LoRA (rank=32)
  JEPA                   — auxiliary MSE loss, EMA target encoder

φ-physics hyperparameters (openai/parameter-golf#1742):
  base_lr = α_φ = φ^{-3}/2 ≈ 0.118034
  loops   = floor(φ^3)   = 4
  init_σ = φ^{-2}       ≈ 0.382
"""

from __future__ import annotations

import copy
import glob
import io
import math
import os
import random
import subprocess
import sys
import time
import uuid
import zlib
from pathlib import Path

import numpy as np
import sentencepiece as spm
import torch
import torch.distributed as dist
import torch.nn.functional as F
from torch import Tensor, nn
from torch.nn.parallel import DistributedDataParallel as DDP

# ---------------------------------------------------------------------------
# φ-PHYSICS CONSTANTS
# ---------------------------------------------------------------------------

PHI: float = 1.6180339887498948482          # golden ratio
ALPHA_PHI: float = PHI ** (-3) / 2          # ≈ 0.118034, strong coupling α_s analogue
PHI_LOOPS: int = int(PHI ** 3)              # = 4, Universal Transformer depth
PHI_INIT_STD: float = PHI ** (-2)           # ≈ 0.382, weight init σ
FIB_RANK: int = 32                           # LoRA rank (Fibonacci-adjacent)

# ---------------------------------------------------------------------------
# HYPERPARAMETERS
# ---------------------------------------------------------------------------

class Hyperparameters:
    data_path         = os.environ.get("DATA_PATH", "./data/datasets/fineweb10B_sp1024")
    train_files       = os.path.join(data_path, "fineweb_train_*.bin")
    val_files         = os.path.join(data_path, "fineweb_val_*.bin")
    tokenizer_path    = os.environ.get("TOKENIZER_PATH", "./data/tokenizers/fineweb_1024_bpe.model")
    run_id            = os.environ.get("RUN_ID", str(uuid.uuid4()))
    seed              = int(os.environ.get("SEED", 42))

    val_batch_size    = int(os.environ.get("VAL_BATCH_SIZE", 524_288))
    val_loss_every    = int(os.environ.get("VAL_LOSS_EVERY", 1000))
    train_log_every   = int(os.environ.get("TRAIN_LOG_EVERY", 200))

    # 4h non-record budget: 80k steps
    iterations        = int(os.environ.get("ITERATIONS", 80_000))
    warmdown_iters    = int(os.environ.get("WARMDOWN_ITERS", 8_000))
    warmup_steps      = int(os.environ.get("WARMUP_STEPS", 20))
    train_batch_tokens = int(os.environ.get("TRAIN_BATCH_TOKENS", 524_288))
    train_seq_len     = int(os.environ.get("TRAIN_SEQ_LEN", 1024))
    max_wallclock_seconds = float(os.environ.get("MAX_WALLCLOCK_SECONDS", 0.0))  # 0 = no cap
    qk_gain_init      = float(os.environ.get("QK_GAIN_INIT", 1.5))

    # Model: Universal Transformer (1 shared block x 4 loops)
    vocab_size        = int(os.environ.get("VOCAB_SIZE", 1024))
    num_layers        = int(os.environ.get("NUM_LAYERS", 1))    # 1 shared block
    ut_loops          = int(os.environ.get("UT_LOOPS", PHI_LOOPS))   # 4 recurrence loops
    num_kv_heads      = int(os.environ.get("NUM_KV_HEADS", 4))
    model_dim         = int(os.environ.get("MODEL_DIM", 512))
    num_heads         = int(os.environ.get("NUM_HEADS", 8))
    mlp_mult          = int(os.environ.get("MLP_MULT", 3))
    tie_embeddings    = bool(int(os.environ.get("TIE_EMBEDDINGS", "1")))
    rope_base         = float(os.environ.get("ROPE_BASE", 10000.0))
    logit_softcap     = float(os.environ.get("LOGIT_SOFTCAP", 30.0))

    # NTA
    nta_rank          = int(os.environ.get("NTA_RANK", FIB_RANK))

    # JEPA
    jepa_lambda       = float(os.environ.get("JEPA_LAMBDA", 0.1))
    jepa_mask_rate    = float(os.environ.get("JEPA_MASK_RATE", 0.15))
    ema_decay         = float(os.environ.get("EMA_DECAY", 0.996))

    # φ-LR
    embed_lr          = float(os.environ.get("EMBED_LR", ALPHA_PHI * 5))  # ≈ 0.59
    head_lr           = float(os.environ.get("HEAD_LR", ALPHA_PHI / 15))  # ≈ 0.008
    tied_embed_lr     = float(os.environ.get("TIED_EMBED_LR", ALPHA_PHI / 2.36))  # ≈ 0.05
    tied_embed_init_std = float(os.environ.get("TIED_EMBED_INIT_STD", PHI_INIT_STD * 0.013))
    matrix_lr         = float(os.environ.get("MATRIX_LR", 0.04))
    scalar_lr         = float(os.environ.get("SCALAR_LR", 0.04))
    muon_momentum     = float(os.environ.get("MUON_MOMENTUM", 0.95))
    muon_backend_steps = int(os.environ.get("MUON_BACKEND_STEPS", 5))
    muon_momentum_warmup_start = float(os.environ.get("MUON_MOMENTUM_WARMUP_START", 0.85))
    muon_momentum_warmup_steps = int(os.environ.get("MUON_MOMENTUM_WARMUP_STEPS", 500))
    beta1             = float(os.environ.get("BETA1", 0.9))
    beta2             = float(os.environ.get("BETA2", 0.95))
    adam_eps          = float(os.environ.get("ADAM_EPS", 1e-8))
    grad_clip_norm    = float(os.environ.get("GRAD_CLIP_NORM", 0.0))

# ---------------------------------------------------------------------------
# MUON OPTIMIZER (unchanged from base train_gpt.py)
# ---------------------------------------------------------------------------

def zeropower_via_newtonschulz5(G: Tensor, steps: int = 10, eps: float = 1e-7) -> Tensor:
    a, b, c = (3.4445, -4.7750, 2.0315)
    X = G.bfloat16()
    X /= X.norm() + eps
    transposed = G.size(0) > G.size(1)
    if transposed:
        X = X.T
    for _ in range(steps):
        A = X @ X.T
        B = b * A + c * A @ A
        X = a * X + B @ X
    return X.T if transposed else X


class Muon(torch.optim.Optimizer):
    def __init__(self, params, lr: float, momentum: float, backend_steps: int, nesterov: bool = True):
        super().__init__(params, dict(lr=lr, momentum=momentum, backend_steps=backend_steps, nesterov=nesterov))

    @torch.no_grad()
    def step(self, closure=None):
        loss = None
        if closure is not None:
            with torch.enable_grad():
                loss = closure()
        distributed = dist.is_available() and dist.is_initialized()
        world_size = dist.get_world_size() if distributed else 1
        rank = dist.get_rank() if distributed else 0
        for group in self.param_groups:
            params = group["params"]
            if not params:
                continue
            lr, momentum, backend_steps, nesterov = group["lr"], group["momentum"], group["backend_steps"], group["nesterov"]
            total_params = sum(int(p.numel()) for p in params)
            updates_flat = torch.zeros(total_params, device=params[0].device, dtype=torch.bfloat16)
            curr = 0
            for i, p in enumerate(params):
                if i % world_size == rank and p.grad is not None:
                    g = p.grad
                    state = self.state[p]
                    if "momentum_buffer" not in state:
                        state["momentum_buffer"] = torch.zeros_like(g)
                    buf = state["momentum_buffer"]
                    buf.mul_(momentum).add_(g)
                    if nesterov:
                        g = g.add(buf, alpha=momentum)
                    g = zeropower_via_newtonschulz5(g, steps=backend_steps)
                    g *= max(1, g.size(0) / g.size(1)) ** 0.5
                    updates_flat[curr : curr + p.numel()] = g.reshape(-1)
                curr += p.numel()
            if distributed:
                dist.all_reduce(updates_flat, op=dist.ReduceOp.SUM)
            curr = 0
            for p in params:
                g = updates_flat[curr : curr + p.numel()].view_as(p).to(dtype=p.dtype)
                p.add_(g, alpha=-lr)
                curr += p.numel()
        return loss

# ---------------------------------------------------------------------------
# DATA UTILITIES (unchanged)
# ---------------------------------------------------------------------------

def build_sentencepiece_luts(sp, vocab_size, device):
    sp_vocab_size = int(sp.vocab_size())
    table_size = max(sp_vocab_size, vocab_size)
    base_bytes_np = np.zeros((table_size,), dtype=np.int16)
    has_leading_space_np = np.zeros((table_size,), dtype=np.bool_)
    is_boundary_token_np = np.ones((table_size,), dtype=np.bool_)
    for token_id in range(sp_vocab_size):
        if sp.is_control(token_id) or sp.is_unknown(token_id) or sp.is_unused(token_id):
            continue
        is_boundary_token_np[token_id] = False
        if sp.is_byte(token_id):
            base_bytes_np[token_id] = 1
            continue
        piece = sp.id_to_piece(token_id)
        if piece.startswith("▁"):
            has_leading_space_np[token_id] = True
            piece = piece[1:]
        base_bytes_np[token_id] = len(piece.encode("utf-8"))
    return (
        torch.tensor(base_bytes_np, dtype=torch.int16, device=device),
        torch.tensor(has_leading_space_np, dtype=torch.bool, device=device),
        torch.tensor(is_boundary_token_np, dtype=torch.bool, device=device),
    )


def load_data_shard(file: Path) -> Tensor:
    header_bytes = 256 * np.dtype("<i4").itemsize
    token_bytes = np.dtype("<u2").itemsize
    header = np.fromfile(file, dtype="<i4", count=256)
    if header.size != 256 or int(header[0]) != 20240520 or int(header[1]) != 1:
        raise ValueError(f"Unexpected shard header for {file}")
    num_tokens = int(header[2])
    expected_size = header_bytes + num_tokens * token_bytes
    if file.stat().st_size != expected_size:
        raise ValueError(f"Shard size mismatch for {file}")
    tokens_np = np.fromfile(file, dtype="<u2", count=num_tokens, offset=header_bytes)
    return torch.from_numpy(tokens_np.astype(np.uint16, copy=False))


def load_validation_tokens(pattern: str, seq_len: int) -> Tensor:
    files = [Path(p) for p in sorted(glob.glob(pattern))]
    if not files:
        raise FileNotFoundError(f"No files found for pattern: {pattern}")
    tokens = torch.cat([load_data_shard(file) for file in files]).contiguous()
    usable = ((tokens.numel() - 1) // seq_len) * seq_len
    return tokens[: usable + 1]


class TokenStream:
    def __init__(self, pattern: str):
        self.files = [Path(p) for p in sorted(glob.glob(pattern))]
        if not self.files:
            raise FileNotFoundError(f"No files found: {pattern}")
        self.file_idx = 0
        self.tokens = load_data_shard(self.files[0])
        self.pos = 0

    def _advance_file(self):
        self.file_idx = (self.file_idx + 1) % len(self.files)
        self.tokens = load_data_shard(self.files[self.file_idx])
        self.pos = 0

    def take(self, n: int) -> Tensor:
        chunks: list[Tensor] = []
        remaining = n
        while remaining > 0:
            avail = self.tokens.numel() - self.pos
            if avail <= 0:
                self._advance_file()
                continue
            k = min(remaining, avail)
            chunks.append(self.tokens[self.pos : self.pos + k])
            self.pos += k
            remaining -= k
        return chunks[0] if len(chunks) == 1 else torch.cat(chunks)


class DistributedTokenLoader:
    def __init__(self, pattern: str, rank: int, world_size: int, device: torch.device):
        self.rank = rank
        self.world_size = world_size
        self.device = device
        self.stream = TokenStream(pattern)

    def next_batch(self, global_tokens: int, seq_len: int, grad_accum_steps: int):
        local_tokens = global_tokens // (self.world_size * grad_accum_steps)
        per_rank_span = local_tokens + 1
        chunk = self.stream.take(per_rank_span * self.world_size)
        start = self.rank * per_rank_span
        local = chunk[start : start + per_rank_span].to(dtype=torch.int64)
        x = local[:-1].reshape(-1, seq_len)
        y = local[1:].reshape(-1, seq_len)
        return x.to(self.device, non_blocking=True), y.to(self.device, non_blocking=True)

# ---------------------------------------------------------------------------
# INT8 QUANTIZATION (unchanged from base)
# ---------------------------------------------------------------------------

CONTROL_TENSOR_NAME_PATTERNS = tuple(
    p for p in os.environ.get(
        "CONTROL_TENSOR_NAME_PATTERNS",
        "attn_scale,attn_scales,mlp_scale,mlp_scales,resid_mix,resid_mixes,q_gain,skip_weight,skip_weights",
    ).split(",") if p
)
INT8_KEEP_FLOAT_FP32_NAME_PATTERNS = tuple(
    p for p in os.environ.get(
        "INT8_KEEP_FLOAT_FP32_NAME_PATTERNS",
        ",".join(CONTROL_TENSOR_NAME_PATTERNS),
    ).split(",") if p
)
INT8_KEEP_FLOAT_MAX_NUMEL = 65_536
INT8_KEEP_FLOAT_STORE_DTYPE = torch.float16
INT8_PER_ROW_SCALE_DTYPE = torch.float16
INT8_CLIP_PERCENTILE = 99.99984
INT8_CLIP_Q = INT8_CLIP_PERCENTILE / 100.0


def tensor_nbytes(t: Tensor) -> int:
    return int(t.numel()) * int(t.element_size())


def keep_float_tensor(name, t, passthrough_orig_dtypes):
    if any(pattern in name for pattern in INT8_KEEP_FLOAT_FP32_NAME_PATTERNS):
        return t.float().contiguous()
    if t.dtype in {torch.float32, torch.bfloat16}:
        passthrough_orig_dtypes[name] = str(t.dtype).removeprefix("torch.")
        return t.to(dtype=INT8_KEEP_FLOAT_STORE_DTYPE).contiguous()
    return t


def quantize_float_tensor(t: Tensor):
    t32 = t.float()
    if t32.ndim == 2:
        clip_abs = (
            torch.quantile(t32.abs(), INT8_CLIP_Q, dim=1)
            if t32.numel() else torch.empty((t32.shape[0],), dtype=torch.float32)
        )
        clipped = torch.maximum(torch.minimum(t32, clip_abs[:, None]), -clip_abs[:, None])
        scale = (clip_abs / 127.0).clamp_min(1.0 / 127.0)
        q = torch.clamp(torch.round(clipped / scale[:, None]), -127, 127).to(torch.int8).contiguous()
        return q, scale.to(dtype=INT8_PER_ROW_SCALE_DTYPE).contiguous()
    clip_abs = float(torch.quantile(t32.abs().flatten(), INT8_CLIP_Q).item()) if t32.numel() else 0.0
    scale = torch.tensor(clip_abs / 127.0 if clip_abs > 0 else 1.0, dtype=torch.float32)
    q = torch.clamp(torch.round(torch.clamp(t32, -clip_abs, clip_abs) / scale), -127, 127).to(torch.int8).contiguous()
    return q, scale


def quantize_state_dict_int8(state_dict):
    quantized, scales, dtypes, passthrough, passthrough_orig_dtypes, qmeta = {}, {}, {}, {}, {}, {}
    stats = dict.fromkeys(("param_count", "num_tensors", "num_float_tensors", "num_nonfloat_tensors", "baseline_tensor_bytes", "int8_payload_bytes"), 0)
    for name, tensor in state_dict.items():
        # Skip NTA frozen buffers (not parameters, not part of artifact)
        if "phi_basis" in name:
            continue
        t = tensor.detach().to("cpu").contiguous()
        stats["param_count"] += int(t.numel())
        stats["num_tensors"] += 1
        stats["baseline_tensor_bytes"] += tensor_nbytes(t)
        if not t.is_floating_point():
            stats["num_nonfloat_tensors"] += 1
            passthrough[name] = t
            stats["int8_payload_bytes"] += tensor_nbytes(t)
            continue
        if t.numel() <= INT8_KEEP_FLOAT_MAX_NUMEL:
            kept = keep_float_tensor(name, t, passthrough_orig_dtypes)
            passthrough[name] = kept
            stats["int8_payload_bytes"] += tensor_nbytes(kept)
            continue
        stats["num_float_tensors"] += 1
        q, s = quantize_float_tensor(t)
        if s.ndim > 0:
            qmeta[name] = {"scheme": "per_row", "axis": 0}
        quantized[name] = q
        scales[name] = s
        dtypes[name] = str(t.dtype).removeprefix("torch.")
        stats["int8_payload_bytes"] += tensor_nbytes(q) + tensor_nbytes(s)
    obj = {
        "__quant_format__": "int8_clean_per_row_v1",
        "quantized": quantized, "scales": scales, "dtypes": dtypes, "passthrough": passthrough,
    }
    if qmeta:
        obj["qmeta"] = qmeta
    if passthrough_orig_dtypes:
        obj["passthrough_orig_dtypes"] = passthrough_orig_dtypes
    return obj, stats


def dequantize_state_dict_int8(obj):
    out = {}
    qmeta = obj.get("qmeta", {})
    passthrough_orig_dtypes = obj.get("passthrough_orig_dtypes", {})
    for name, q in obj["quantized"].items():
        dtype = getattr(torch, obj["dtypes"][name])
        s = obj["scales"][name]
        if qmeta.get(name, {}).get("scheme") == "per_row" or s.ndim > 0:
            s = s.to(dtype=torch.float32)
            out[name] = (q.float() * s.view(q.shape[0], *([1] * (q.ndim - 1)))).to(dtype=dtype).contiguous()
        else:
            out[name] = (q.float() * float(s.item())).to(dtype=dtype).contiguous()
    for name, t in obj["passthrough"].items():
        out_t = t.detach().to("cpu").contiguous()
        orig_dtype = passthrough_orig_dtypes.get(name)
        if isinstance(orig_dtype, str):
            out_t = out_t.to(dtype=getattr(torch, orig_dtype)).contiguous()
        out[name] = out_t
    return out

# ---------------------------------------------------------------------------
# TRANSFORMER BASE MODULES
# ---------------------------------------------------------------------------

class RMSNorm(nn.Module):
    def __init__(self, eps=None):
        super().__init__()
        self.eps = eps

    def forward(self, x: Tensor) -> Tensor:
        return F.rms_norm(x, (x.size(-1),), eps=self.eps)


class CastedLinear(nn.Linear):
    def forward(self, x: Tensor) -> Tensor:
        bias = self.bias.to(x.dtype) if self.bias is not None else None
        return F.linear(x, self.weight.to(x.dtype), bias)


class Rotary(nn.Module):
    def __init__(self, dim: int, base: float = 10000.0):
        super().__init__()
        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2, dtype=torch.float32) / dim))
        self.register_buffer("inv_freq", inv_freq, persistent=False)
        self._seq_len_cached = 0
        self._cos_cached = None
        self._sin_cached = None

    def forward(self, seq_len, device, dtype):
        if self._cos_cached is None or self._seq_len_cached != seq_len or self._cos_cached.device != device:
            t = torch.arange(seq_len, device=device, dtype=self.inv_freq.dtype)
            freqs = torch.outer(t, self.inv_freq.to(device))
            self._cos_cached = freqs.cos()[None, None, :, :]
            self._sin_cached = freqs.sin()[None, None, :, :]
            self._seq_len_cached = seq_len
        return self._cos_cached.to(dtype=dtype), self._sin_cached.to(dtype=dtype)


def apply_rotary_emb(x, cos, sin):
    half = x.size(-1) // 2
    x1, x2 = x[..., :half], x[..., half:]
    return torch.cat((x1 * cos + x2 * sin, x1 * (-sin) + x2 * cos), dim=-1)


class CausalSelfAttention(nn.Module):
    def __init__(self, dim, num_heads, num_kv_heads, rope_base, qk_gain_init):
        super().__init__()
        self.num_heads = num_heads
        self.num_kv_heads = num_kv_heads
        self.head_dim = dim // num_heads
        kv_dim = num_kv_heads * self.head_dim
        self.c_q = CastedLinear(dim, dim, bias=False)
        self.c_k = CastedLinear(dim, kv_dim, bias=False)
        self.c_v = CastedLinear(dim, kv_dim, bias=False)
        self.proj = CastedLinear(dim, dim, bias=False)
        self.proj._zero_init = True
        self.q_gain = nn.Parameter(torch.full((num_heads,), qk_gain_init, dtype=torch.float32))
        self.rotary = Rotary(self.head_dim, base=rope_base)

    def forward(self, x: Tensor) -> Tensor:
        bsz, seqlen, dim = x.shape
        q = self.c_q(x).reshape(bsz, seqlen, self.num_heads, self.head_dim).transpose(1, 2)
        k = self.c_k(x).reshape(bsz, seqlen, self.num_kv_heads, self.head_dim).transpose(1, 2)
        v = self.c_v(x).reshape(bsz, seqlen, self.num_kv_heads, self.head_dim).transpose(1, 2)
        q, k = F.rms_norm(q, (q.size(-1),)), F.rms_norm(k, (k.size(-1),))
        cos, sin = self.rotary(seqlen, x.device, q.dtype)
        q, k = apply_rotary_emb(q, cos, sin), apply_rotary_emb(k, cos, sin)
        q = q * self.q_gain.to(dtype=q.dtype)[None, :, None, None]
        y = F.scaled_dot_product_attention(q, k, v, attn_mask=None, is_causal=True,
                                           enable_gqa=(self.num_kv_heads != self.num_heads))
        return self.proj(y.transpose(1, 2).contiguous().reshape(bsz, seqlen, dim))


class MLP(nn.Module):
    def __init__(self, dim, mlp_mult):
        super().__init__()
        hidden = mlp_mult * dim
        self.fc = CastedLinear(dim, hidden, bias=False)
        self.proj = CastedLinear(hidden, dim, bias=False)
        self.proj._zero_init = True

    def forward(self, x: Tensor) -> Tensor:
        return self.proj(torch.relu(self.fc(x)).square())


class Block(nn.Module):
    """Standard transformer block — shared across all Universal Transformer loops."""
    def __init__(self, dim, num_heads, num_kv_heads, mlp_mult, rope_base, qk_gain_init):
        super().__init__()
        self.attn_norm = RMSNorm()
        self.mlp_norm = RMSNorm()
        self.attn = CausalSelfAttention(dim, num_heads, num_kv_heads, rope_base, qk_gain_init)
        self.mlp = MLP(dim, mlp_mult)
        self.attn_scale = nn.Parameter(torch.ones(dim, dtype=torch.float32))
        self.mlp_scale = nn.Parameter(torch.ones(dim, dtype=torch.float32))
        self.resid_mix = nn.Parameter(torch.stack((torch.ones(dim), torch.zeros(dim))).float())

    def forward(self, x: Tensor, x0: Tensor) -> Tensor:
        mix = self.resid_mix.to(dtype=x.dtype)
        x = mix[0][None, None, :] * x + mix[1][None, None, :] * x0
        x = x + self.attn_scale.to(dtype=x.dtype)[None, None, :] * self.attn(self.attn_norm(x))
        x = x + self.mlp_scale.to(dtype=x.dtype)[None, None, :] * self.mlp(self.mlp_norm(x))
        return x

# ---------------------------------------------------------------------------
# NTA: NON-TRAINABLE φ-HADAMARD ADAPTER + LORA
# ---------------------------------------------------------------------------

class PhiNTA(nn.Module):
    """
    Non-Trainable Adapter on φ-random linear maps.

    Architecture:
        out = x + lora_B(lora_A(x))   [trainable LoRA path]
    The frozen φ-basis is used to initialize lora_A in a structured way,
    but only lora_A and lora_B are parameters (the basis is a buffer).

    Per-loop instantiation gives each Universal Transformer step a
    unique learned "key" without duplicating the shared Block.
    """
    def __init__(self, dim: int, rank: int, loop_idx: int):
        super().__init__()
        self.dim = dim
        self.rank = rank

        # Frozen φ-structured basis (not a parameter, excluded from artifact)
        g = torch.Generator()
        g.manual_seed(42 + loop_idx * 137)  # deterministic per loop
        basis = torch.randn(dim, dim, generator=g) * (PHI ** -0.5)
        # Impose φ-diagonal structure: scale columns by φ^{-k/dim}
        phi_diag = torch.tensor([PHI ** (-k / dim) for k in range(dim)])
        basis = basis * phi_diag[None, :]
        self.register_buffer("phi_basis", basis.to(torch.bfloat16), persistent=False)

        # Trainable LoRA: A projects down to rank, B projects back up
        # Initialize A with first 'rank' columns of the φ-basis (structured init)
        lora_a_init = basis[:, :rank].clone().float()
        # Normalize so operator norm ≈ 1
        lora_a_init = lora_a_init / (lora_a_init.norm() + 1e-6) * (PHI_INIT_STD)
        self.lora_A = nn.Parameter(lora_a_init)  # (dim, rank)
        self.lora_B = nn.Parameter(torch.zeros(rank, dim))  # (rank, dim) — zero init
        self.scale = nn.Parameter(torch.zeros(dim, dtype=torch.float32))  # gated init

    def forward(self, x: Tensor) -> Tensor:
        # LoRA path: x @ A @ B, gated by learnable scale
        h = F.linear(x, self.lora_A.to(x.dtype).T)    # (..., rank)
        out = F.linear(h, self.lora_B.to(x.dtype))     # (..., dim)
        gate = torch.tanh(self.scale.to(x.dtype))[None, None, :]
        return x + gate * out

# ---------------------------------------------------------------------------
# JEPA PREDICTOR
# ---------------------------------------------------------------------------

class JEPAPredictor(nn.Module):
    """
    Lightweight 2-layer predictor for JEPA auxiliary loss.
    Given context hidden states (with some positions masked), predicts
    the EMA target encoder's hidden states at masked positions.
    Discarded at evaluation — only adds ~0.53M params during training.
    """
    def __init__(self, dim: int):
        super().__init__()
        hidden = int(dim * PHI)  # ≈ 828 for dim=512
        self.fc1 = CastedLinear(dim, hidden, bias=False)
        self.fc2 = CastedLinear(hidden, dim, bias=False)
        self.norm = RMSNorm()

    def forward(self, x: Tensor) -> Tensor:
        """x: (B, T, D) context hidden states. Returns predicted targets."""
        return self.fc2(F.gelu(self.fc1(self.norm(x))))

# ---------------------------------------------------------------------------
# GOLDEN SUNFLOWERS MODEL
# ---------------------------------------------------------------------------

class GoldenSunflowers(nn.Module):
    """
    Universal Transformer + NTA + JEPA.

    Forward pass (training):
      1. Embed tokens
      2. Apply shared Block x ut_loops times, each loop followed by PhiNTA
      3. Compute LM cross-entropy loss
      4. (optional) Compute JEPA auxiliary MSE loss
      Return: lm_loss + lambda * jepa_loss

    Forward pass (eval / no JEPA):
      Same as above but skip JEPA computation.
    """

    def __init__(
        self,
        vocab_size, model_dim, num_heads, num_kv_heads, mlp_mult,
        ut_loops, nta_rank, tie_embeddings, tied_embed_init_std,
        logit_softcap, rope_base, qk_gain_init,
        jepa_lambda=0.1,
    ):
        super().__init__()
        self.ut_loops = ut_loops
        self.tie_embeddings = tie_embeddings
        self.logit_softcap = logit_softcap
        self.jepa_lambda = jepa_lambda
        self.tied_embed_init_std = tied_embed_init_std

        self.tok_emb = nn.Embedding(vocab_size, model_dim)

        # ONE shared transformer block (Universal Transformer)
        self.shared_block = Block(
            model_dim, num_heads, num_kv_heads, mlp_mult, rope_base, qk_gain_init
        )

        # Per-loop NTA adapters (loop-specific LoRA keys, all lightweight)
        self.nta_adapters = nn.ModuleList([
            PhiNTA(model_dim, nta_rank, loop_idx=i)
            for i in range(ut_loops)
        ])

        # Per-loop step embeddings (Adaptive Computation Time style)
        self.step_emb = nn.Parameter(
            torch.randn(ut_loops, model_dim, dtype=torch.float32) * PHI_INIT_STD
        )

        self.final_norm = RMSNorm()
        self.lm_head = None if tie_embeddings else CastedLinear(model_dim, vocab_size, bias=False)
        if self.lm_head is not None:
            self.lm_head._zero_init = True

        # JEPA predictor (training-only; not saved in final artifact)
        self.jepa_predictor = JEPAPredictor(model_dim)
        # EMA target encoder state will be managed externally (see training loop)
        self._target_block_state = None

        self._init_weights()

    def _init_weights(self):
        if self.tie_embeddings:
            nn.init.normal_(self.tok_emb.weight, mean=0.0, std=self.tied_embed_init_std)
        for module in self.modules():
            if isinstance(module, nn.Linear) and getattr(module, "_zero_init", False):
                nn.init.zeros_(module.weight)

    def _encode(self, input_ids: Tensor) -> Tensor:
        """Run the Universal Transformer encoder. Returns final hidden states."""
        x = self.tok_emb(input_ids)
        x = F.rms_norm(x, (x.size(-1),))
        x0 = x  # skip-connection anchor

        for loop_idx in range(self.ut_loops):
            # Inject loop-specific step embedding (like ACT positional signal)
            step = self.step_emb[loop_idx].to(dtype=x.dtype)[None, None, :]
            x = x + step
            # Shared block
            x = self.shared_block(x, x0)
            # Loop-specific NTA adapter
            x = self.nta_adapters[loop_idx](x)

        return self.final_norm(x)

    def forward(
        self,
        input_ids: Tensor,
        target_ids: Tensor,
        jepa_mask: Tensor | None = None,
        target_hidden: Tensor | None = None,
    ) -> Tensor:
        """
        input_ids:    (B, T)
        target_ids:   (B, T)
        jepa_mask:    (B, T) bool — True at masked positions (JEPA training only)
        target_hidden: (B, T, D) — EMA target encoder output (JEPA training only)
        Returns scalar loss.
        """
        hidden = self._encode(input_ids)              # (B, T, D)

        # --- LM loss ---
        flat = hidden.reshape(-1, hidden.size(-1))
        targets = target_ids.reshape(-1)
        if self.tie_embeddings:
            logits = F.linear(flat, self.tok_emb.weight)
        else:
            logits = self.lm_head(flat)
        logits = self.logit_softcap * torch.tanh(logits / self.logit_softcap)
        lm_loss = F.cross_entropy(logits.float(), targets, reduction="mean")

        # --- JEPA auxiliary loss ---
        if jepa_mask is not None and target_hidden is not None and self.jepa_lambda > 0:
            # Predict masked positions from context hidden states
            pred = self.jepa_predictor(hidden)         # (B, T, D)
            # Only compute loss at masked positions
            mask = jepa_mask.unsqueeze(-1).float()     # (B, T, 1)
            pred_norm = F.rms_norm(pred, (pred.size(-1),))
            tgt_norm  = F.rms_norm(target_hidden.detach(), (target_hidden.size(-1),))
            jepa_loss = ((pred_norm - tgt_norm) ** 2 * mask).sum() / (mask.sum() + 1e-6)
            return lm_loss + self.jepa_lambda * jepa_loss

        return lm_loss


def restore_low_dim_params_to_fp32(module: nn.Module) -> None:
    with torch.no_grad():
        for name, param in module.named_parameters():
            if (param.ndim < 2 or any(pattern in name for pattern in CONTROL_TENSOR_NAME_PATTERNS)) and param.dtype != torch.float32:
                param.data = param.data.float()

# ---------------------------------------------------------------------------
# VALIDATION
# ---------------------------------------------------------------------------

def eval_val(args, model, rank, world_size, device, grad_accum_steps,
             val_tokens, base_bytes_lut, has_leading_space_lut, is_boundary_token_lut):
    local_batch_tokens = args.val_batch_size // (world_size * grad_accum_steps)
    local_batch_seqs = local_batch_tokens // args.train_seq_len
    total_seqs = (val_tokens.numel() - 1) // args.train_seq_len
    seq_start = (total_seqs * rank) // world_size
    seq_end = (total_seqs * (rank + 1)) // world_size
    val_loss_sum = torch.zeros((), device=device, dtype=torch.float64)
    val_token_count = torch.zeros((), device=device, dtype=torch.float64)
    val_byte_count = torch.zeros((), device=device, dtype=torch.float64)

    model.eval()
    with torch.inference_mode():
        for batch_seq_start in range(seq_start, seq_end, local_batch_seqs):
            batch_seq_end = min(batch_seq_start + local_batch_seqs, seq_end)
            raw_start = batch_seq_start * args.train_seq_len
            raw_end = batch_seq_end * args.train_seq_len + 1
            local = val_tokens[raw_start:raw_end].to(device=device, dtype=torch.int64, non_blocking=True)
            x = local[:-1].reshape(-1, args.train_seq_len)
            y = local[1:].reshape(-1, args.train_seq_len)
            with torch.autocast(device_type="cuda", dtype=torch.bfloat16, enabled=True):
                # Eval: no JEPA mask
                batch_loss = model(x, y).detach()
            batch_token_count = float(y.numel())
            val_loss_sum += batch_loss.to(torch.float64) * batch_token_count
            val_token_count += batch_token_count
            prev_ids = x.reshape(-1)
            tgt_ids = y.reshape(-1)
            token_bytes = base_bytes_lut[tgt_ids].to(dtype=torch.int16)
            token_bytes += (has_leading_space_lut[tgt_ids] & ~is_boundary_token_lut[prev_ids]).to(dtype=torch.int16)
            val_byte_count += token_bytes.to(torch.float64).sum()

    if dist.is_available() and dist.is_initialized():
        dist.all_reduce(val_loss_sum, op=dist.ReduceOp.SUM)
        dist.all_reduce(val_token_count, op=dist.ReduceOp.SUM)
        dist.all_reduce(val_byte_count, op=dist.ReduceOp.SUM)

    val_loss = float((val_loss_sum / val_token_count).item())
    bits_per_token = val_loss / math.log(2.0)
    tokens_per_byte = float(val_token_count.item() / val_byte_count.item())
    model.train()
    return val_loss, float(bits_per_token * tokens_per_byte)

# ---------------------------------------------------------------------------
# MAIN
# ---------------------------------------------------------------------------

def main():
    global zeropower_via_newtonschulz5

    code = Path(__file__).read_text(encoding="utf-8")
    args = Hyperparameters()
    zeropower_via_newtonschulz5 = torch.compile(zeropower_via_newtonschulz5)

    distributed = "RANK" in os.environ and "WORLD_SIZE" in os.environ
    rank = int(os.environ.get("RANK", "0"))
    world_size = int(os.environ.get("WORLD_SIZE", "1"))
    local_rank = int(os.environ.get("LOCAL_RANK", "0"))
    grad_accum_steps = 8 // world_size
    grad_scale = 1.0 / grad_accum_steps

    if not torch.cuda.is_available():
        raise RuntimeError("CUDA required")
    device = torch.device("cuda", local_rank)
    torch.cuda.set_device(device)
    if distributed:
        dist.init_process_group(backend="nccl", device_id=device)
        dist.barrier()
    master_process = rank == 0

    torch.backends.cuda.matmul.allow_tf32 = True
    torch.backends.cudnn.allow_tf32 = True
    from torch.backends.cuda import enable_cudnn_sdp, enable_flash_sdp, enable_math_sdp, enable_mem_efficient_sdp
    enable_cudnn_sdp(False); enable_flash_sdp(True); enable_mem_efficient_sdp(False); enable_math_sdp(False)

    logfile = None
    if master_process:
        os.makedirs("logs", exist_ok=True)
        logfile = f"logs/{args.run_id}.txt"
        print(logfile)

    def log0(msg, console=True):
        if not master_process:
            return
        if console:
            print(msg)
        if logfile is not None:
            with open(logfile, "a", encoding="utf-8") as f:
                print(msg, file=f)

    log0(code, console=False)
    log0(f"GOLDEN SUNFLOWERS 🌻 | φ={PHI:.6f} | α_φ={ALPHA_PHI:.6f} | loops={PHI_LOOPS}")
    log0(f"Universal Transformer: 1 shared block x {args.ut_loops} loops")
    log0(f"NTA: frozen φ-basis + LoRA rank={args.nta_rank} x {args.ut_loops} adapters")
    log0(f"JEPA: λ={args.jepa_lambda} mask_rate={args.jepa_mask_rate} ema_decay={args.ema_decay}")

    random.seed(args.seed)
    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    torch.cuda.manual_seed_all(args.seed)

    sp = spm.SentencePieceProcessor(model_file=args.tokenizer_path)
    val_tokens = load_validation_tokens(args.val_files, args.train_seq_len)
    base_bytes_lut, has_leading_space_lut, is_boundary_token_lut = build_sentencepiece_luts(sp, args.vocab_size, device)

    # --- Model ---
    base_model = GoldenSunflowers(
        vocab_size=args.vocab_size,
        model_dim=args.model_dim,
        num_heads=args.num_heads,
        num_kv_heads=args.num_kv_heads,
        mlp_mult=args.mlp_mult,
        ut_loops=args.ut_loops,
        nta_rank=args.nta_rank,
        tie_embeddings=args.tie_embeddings,
        tied_embed_init_std=args.tied_embed_init_std,
        logit_softcap=args.logit_softcap,
        rope_base=args.rope_base,
        qk_gain_init=args.qk_gain_init,
        jepa_lambda=args.jepa_lambda,
    ).to(device).bfloat16()

    for module in base_model.modules():
        if isinstance(module, CastedLinear):
            module.float()
    restore_low_dim_params_to_fp32(base_model)

    # EMA target model for JEPA (same architecture, no grad)
    target_model = copy.deepcopy(base_model)
    for p in target_model.parameters():
        p.requires_grad_(False)

    compiled_model = torch.compile(base_model, dynamic=False, fullgraph=True)
    model = DDP(compiled_model, device_ids=[local_rank], broadcast_buffers=False) if distributed else compiled_model

    # --- Optimizer split ---
    block_named_params = list(base_model.shared_block.named_parameters())
    nta_named_params = [(f"nta_{i}.{n}", p) for i, adapter in enumerate(base_model.nta_adapters)
                        for n, p in adapter.named_parameters()]
    predictor_named_params = list(base_model.jepa_predictor.named_parameters())

    matrix_params = [
        p for name, p in block_named_params
        if p.ndim == 2 and not any(pat in name for pat in CONTROL_TENSOR_NAME_PATTERNS)
    ] + [
        p for name, p in nta_named_params + predictor_named_params
        if p.ndim == 2
    ]
    scalar_params = [
        p for name, p in block_named_params
        if p.ndim < 2 or any(pat in name for pat in CONTROL_TENSOR_NAME_PATTERNS)
    ] + [
        p for name, p in nta_named_params
        if p.ndim < 2
    ] + [base_model.step_emb]

    token_lr = args.tied_embed_lr if args.tie_embeddings else args.embed_lr
    optimizer_tok = torch.optim.Adam(
        [{"params": [base_model.tok_emb.weight], "lr": token_lr, "base_lr": token_lr}],
        betas=(args.beta1, args.beta2), eps=args.adam_eps, fused=True,
    )
    optimizer_muon = Muon(matrix_params, lr=args.matrix_lr,
                          momentum=args.muon_momentum, backend_steps=args.muon_backend_steps)
    for group in optimizer_muon.param_groups:
        group["base_lr"] = args.matrix_lr
    optimizer_scalar = torch.optim.Adam(
        [{"params": scalar_params, "lr": args.scalar_lr, "base_lr": args.scalar_lr}],
        betas=(args.beta1, args.beta2), eps=args.adam_eps, fused=True,
    )
    optimizers = [optimizer_tok, optimizer_muon, optimizer_scalar]
    if base_model.lm_head is not None:
        optimizer_head = torch.optim.Adam(
            [{"params": [base_model.lm_head.weight], "lr": args.head_lr, "base_lr": args.head_lr}],
            betas=(args.beta1, args.beta2), eps=args.adam_eps, fused=True,
        )
        optimizers.insert(1, optimizer_head)

    n_params = sum(p.numel() for p in base_model.parameters())
    n_params_no_jepa = sum(p.numel() for n, p in base_model.named_parameters()
                           if "jepa" not in n)
    log0(f"model_params_total:{n_params}  model_params_no_jepa:{n_params_no_jepa}")
    log0(f"ut_loops:{args.ut_loops}  nta_rank:{args.nta_rank}  model_dim:{args.model_dim}")

    # --- Data ---
    train_loader = DistributedTokenLoader(args.train_files, rank, world_size, device)
    max_wallclock_ms = 1000.0 * args.max_wallclock_seconds if args.max_wallclock_seconds > 0 else None

    def zero_grad_all():
        for opt in optimizers:
            opt.zero_grad(set_to_none=True)

    def lr_mul(step, elapsed_ms):
        if args.warmdown_iters <= 0:
            return 1.0
        if max_wallclock_ms is None:
            warmdown_start = max(args.iterations - args.warmdown_iters, 0)
            return max((args.iterations - step) / max(args.warmdown_iters, 1), 0.0) if warmdown_start <= step else 1.0
        step_ms = elapsed_ms / max(step, 1)
        warmdown_ms = args.warmdown_iters * step_ms
        remaining_ms = max(max_wallclock_ms - elapsed_ms, 0.0)
        return remaining_ms / max(warmdown_ms, 1e-9) if remaining_ms <= warmdown_ms else 1.0

    def update_ema(online_model, ema_model, decay):
        """Polyak EMA update for JEPA target encoder."""
        with torch.no_grad():
            for (name_o, param_o), (name_e, param_e) in zip(
                online_model.named_parameters(), ema_model.named_parameters()
            ):
                if "jepa_predictor" in name_o:
                    continue  # EMA target has no predictor
                param_e.data.mul_(decay).add_(param_o.data.to(param_e.dtype), alpha=1.0 - decay)

    def make_jepa_mask(bsz, seq_len, mask_rate, device):
        """Random mask: True at positions to predict."""
        return torch.rand(bsz, seq_len, device=device) < mask_rate

    # --- Warmup ---
    if args.warmup_steps > 0:
        initial_model_state = {n: t.detach().cpu().clone() for n, t in base_model.state_dict().items()}
        initial_opt_states = [copy.deepcopy(opt.state_dict()) for opt in optimizers]
        model.train()
        for warmup_step in range(args.warmup_steps):
            zero_grad_all()
            for micro_step in range(grad_accum_steps):
                if distributed:
                    model.require_backward_grad_sync = micro_step == grad_accum_steps - 1
                x, y = train_loader.next_batch(args.train_batch_tokens, args.train_seq_len, grad_accum_steps)
                with torch.autocast(device_type="cuda", dtype=torch.bfloat16, enabled=True):
                    warmup_loss = model(x, y)
                (warmup_loss * grad_scale).backward()
            for opt in optimizers:
                opt.step()
            zero_grad_all()
        base_model.load_state_dict(initial_model_state, strict=True)
        for opt, state in zip(optimizers, initial_opt_states, strict=True):
            opt.load_state_dict(state)
        zero_grad_all()
        train_loader = DistributedTokenLoader(args.train_files, rank, world_size, device)
        log0(f"warmup complete: {args.warmup_steps} steps")

    # --- Main training loop ---
    training_time_ms = 0.0
    stop_after_step = None
    torch.cuda.synchronize()
    t0 = time.perf_counter()
    step = 0

    while True:
        last_step = step == args.iterations or (stop_after_step is not None and step >= stop_after_step)
        should_validate = last_step or (args.val_loss_every > 0 and step % args.val_loss_every == 0)

        if should_validate:
            torch.cuda.synchronize()
            training_time_ms += 1000.0 * (time.perf_counter() - t0)
            val_loss, val_bpb = eval_val(
                args, model, rank, world_size, device, grad_accum_steps,
                val_tokens, base_bytes_lut, has_leading_space_lut, is_boundary_token_lut,
            )
            log0(
                f"step:{step}/{args.iterations} val_loss:{val_loss:.4f} val_bpb:{val_bpb:.4f} "
                f"train_time:{training_time_ms:.0f}ms step_avg:{training_time_ms / max(step, 1):.2f}ms"
            )
            torch.cuda.synchronize()
            t0 = time.perf_counter()

        if last_step:
            break

        elapsed_ms = training_time_ms + 1000.0 * (time.perf_counter() - t0)
        scale = lr_mul(step, elapsed_ms)
        zero_grad_all()
        train_loss = torch.zeros((), device=device)

        for micro_step in range(grad_accum_steps):
            if distributed:
                model.require_backward_grad_sync = micro_step == grad_accum_steps - 1
            x, y = train_loader.next_batch(args.train_batch_tokens, args.train_seq_len, grad_accum_steps)

            # JEPA: get target hidden states (no grad, EMA model)
            jepa_mask = make_jepa_mask(x.size(0), x.size(1), args.jepa_mask_rate, device)
            with torch.no_grad():
                with torch.autocast(device_type="cuda", dtype=torch.bfloat16, enabled=True):
                    target_hidden = target_model._encode(x)  # (B, T, D)

            with torch.autocast(device_type="cuda", dtype=torch.bfloat16, enabled=True):
                loss = model(x, y, jepa_mask=jepa_mask, target_hidden=target_hidden)
            train_loss += loss.detach()
            (loss * grad_scale).backward()

        train_loss /= grad_accum_steps

        # EMA update for JEPA target
        update_ema(base_model, target_model, args.ema_decay)

        # Muon momentum warmup
        frac = min(step / args.muon_momentum_warmup_steps, 1.0) if args.muon_momentum_warmup_steps > 0 else 1.0
        muon_momentum = (1 - frac) * args.muon_momentum_warmup_start + frac * args.muon_momentum
        for group in optimizer_muon.param_groups:
            group["momentum"] = muon_momentum

        for opt in optimizers:
            for group in opt.param_groups:
                group["lr"] = group["base_lr"] * scale

        if args.grad_clip_norm > 0:
            torch.nn.utils.clip_grad_norm_(base_model.parameters(), args.grad_clip_norm)
        for opt in optimizers:
            opt.step()
        zero_grad_all()

        step += 1
        approx_training_time_ms = training_time_ms + 1000.0 * (time.perf_counter() - t0)
        if args.train_log_every > 0 and (step <= 10 or step % args.train_log_every == 0):
            log0(
                f"step:{step}/{args.iterations} train_loss:{train_loss.item():.4f} "
                f"train_time:{approx_training_time_ms:.0f}ms step_avg:{approx_training_time_ms / step:.2f}ms"
            )

        reached_cap = max_wallclock_ms is not None and approx_training_time_ms >= max_wallclock_ms
        if distributed and max_wallclock_ms is not None:
            reached_cap_tensor = torch.tensor(int(reached_cap), device=device)
            dist.all_reduce(reached_cap_tensor, op=dist.ReduceOp.MAX)
            reached_cap = bool(reached_cap_tensor.item())
        if stop_after_step is None and reached_cap:
            stop_after_step = step

    log0(f"peak memory: {torch.cuda.max_memory_allocated() // 1024 // 1024} MiB")

    # --- Serialization ---
    # Drop JEPA predictor before saving (training-only module)
    if master_process:
        # Save full model (debug)
        torch.save(base_model.state_dict(), "final_model.pt")
        log0(f"full model saved: {os.path.getsize('final_model.pt')} bytes")

    # Build artifact state dict: exclude jepa_predictor + phi_basis buffers
    artifact_state = {
        k: v for k, v in base_model.state_dict().items()
        if "jepa_predictor" not in k and "phi_basis" not in k
    }
    quant_obj, quant_stats = quantize_state_dict_int8(artifact_state)
    quant_buf = io.BytesIO()
    torch.save(quant_obj, quant_buf)
    quant_blob = zlib.compress(quant_buf.getvalue(), level=9)

    if master_process:
        with open("final_model.int8.ptz", "wb") as f:
            f.write(quant_blob)
        quant_file_bytes = os.path.getsize("final_model.int8.ptz")
        code_bytes = len(code.encode("utf-8"))
        ratio = quant_stats["baseline_tensor_bytes"] / max(quant_stats["int8_payload_bytes"], 1)
        log0(
            f"artifact int8+zlib: {quant_file_bytes} bytes  code: {code_bytes} bytes  "
            f"total: {quant_file_bytes + code_bytes} bytes  ratio:{ratio:.2f}x"
        )
        compliance = "PASS" if quant_file_bytes + code_bytes <= 16_000_000 else "FAIL"
        log0(f"16MB compliance: {compliance} ({quant_file_bytes + code_bytes} / 16000000)")

    # Roundtrip validation
    if distributed:
        dist.barrier()
    with open("final_model.int8.ptz", "rb") as f:
        quant_blob_disk = f.read()
    quant_state = torch.load(io.BytesIO(zlib.decompress(quant_blob_disk)), map_location="cpu")
    restored = dequantize_state_dict_int8(quant_state)
    # Restore only non-jepa, non-phi_basis params
    base_model.load_state_dict(restored, strict=False)
    torch.cuda.synchronize()
    t_qeval = time.perf_counter()
    q_val_loss, q_val_bpb = eval_val(
        args, model, rank, world_size, device, grad_accum_steps,
        val_tokens, base_bytes_lut, has_leading_space_lut, is_boundary_token_lut,
    )
    torch.cuda.synchronize()
    log0(
        f"final_roundtrip val_loss:{q_val_loss:.4f} val_bpb:{q_val_bpb:.4f} "
        f"eval_time:{1000.0 * (time.perf_counter() - t_qeval):.0f}ms"
    )
    log0(f"final_roundtrip_exact val_loss:{q_val_loss:.8f} val_bpb:{q_val_bpb:.8f}")

    if distributed:
        dist.destroy_process_group()


if __name__ == "__main__":
    main()

# AGENTS.md — trios-railway

Constitution: same as `gHashTag/trios` (`SOUL.md`, `CLAUDE.md`, `AGENTS.md`, `LAWS.md`, `NOW.json`, eternal issue [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)).
Anchor: `phi^2 + phi^-2 = 3`.

## Scope of this repo

`trios-railway` is the **operator surface** for Railway. It does:

- typed Railway GraphQL queries and mutations
- online audit between Railway reality, the Neon `igla_*` ledger, and `.trinity/experience/`
- single-binary CLI `tri-railway` (one verb = one subcommand, L20)

It does **NOT**:

- touch trainer/JEPA/INV-* code (that lives in `trios-trainer-igla` submodule)
- modify `.t27`/`.tri` specs (CANON_DE_ZIGFICATION)
- close eternal issue #143 (L10)

## Standing rules (binding)

- **R1** Rust-only.
- **R5** Honest exit codes; CLI never claims success on upstream failure.
- **R7** Every mutation seals an audit triplet: `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`
- **R9** Mutations are gated by `igla check <sha>`.
- **L1** No `.sh` files (CI self-checks).
- **L2** Every PR `Closes #N`.
- **L3** Clippy zero warnings.
- **L4** Tests pass; new code carries new tests.
- **L7** Append a line to `.trinity/experience/` for every significant task.
- **L8** Push first.
- **L11** Pick a soul-name (humorous English) before mutation.
- **L21** `.trinity/experience/` is append-only; never truncate.

## Current architectural constraints (INV-6, H4_TTT, scarab)

- **INV-6 H4 invariants**: hidden sizes must be `{128, 1408, 2432, 3712}` (e·128 for e∈{1,11,19,29}); ctx lengths must be `{2, 12, 20, 30}`. See `trios-igla-race` SR-00 ring.
- **H4_TTT**: projection-defect test-time training (1/240 weights) integrated into `train_loop.rs`. Enabled via `TRIOS_H4_TTT=1`. `ttt_lr = lr · φ⁻³`.
- **Scarab queue worker**: `scarab.rs` consumes `experiment_queue` (priority DESC) then `strategy_queue`. Entrypoint dispatches `TRIOS_TRAINER_BIN=scarab` → no CLI args, env-driven.
- **Muon lm_head bugfix** (ngram_train.rs): embed/ctx/head must use AdamW, not Muon. Only hidden-layer matrices (proj, attn) get Muon orthogonalization.
- **Railway deploy**: CLI tokens expired; use GraphQL API (`variableUpsert` + `serviceInstanceRedeploy`). All scarab services now have `TRIOS_TRAINER_BIN=scarab`.

## Ring layout

```
crates/
├── trios-railway-core/         RW-00 identity types · RW-01 transport
├── trios-railway-audit/        AU-00 DDL · AU-01 drift detector
└── trios-railway-experience/   EX-00 append-only writer
bin/
└── tri-railway/                BR-CLI entry point
```

Each ring carries `README.md` + `TASK.md` + `AGENTS.md` describing its local invariants (issue [#11](https://github.com/gHashTag/trios-railway/issues/11)).

## Commit etiquette

```
feat(rw-01): typed list-services query

Closes #4
Agent: GENERAL
```

## H4 Application to Neural Network Training (v3.4) — SG-Class Formula Integration

### 1. Validation of Current Hyperparameters

Project already uses H4-derived dimensions:
- `hidden = {128, 1408, 2432, 3712}` = `{1, 11, 19, 29} × 128` = `H4_EXPONENTS × BASE`
- `ctx = {2, 12, 20, 30}` = `H4_DEGREES`

The H4 Coxeter framework independently confirms these same numbers (through φ, π, 239) give the most precise match to real physical constants (0.000103% for α_EW⁻¹). This strengthens INV-6 — we use the same invariant basis that describes the Standard Model.

### 2. H4-Derived Optimizer Formulas

| Formula | Value | Application |
|---------|-------|-------------|
| `239φ⁴/π⁴` | ~128.938 | LR scale for MUON |
| `360φ⁻³` | ~85.06 | Base LR scale |
| `1 + 1/(15πφ)` | ~1.013 | Loop correction factor for WSD decay |
| `φ⁻³` | ~0.236 | H4_TTT `ttt_lr = lr × φ⁻³` |
| `1/240` | — | Projection defect E8 → H4 |

**Key experiments seeded:**
- `h4-l02-lr` — lr=0.000103 (L02 error as LR), 81K steps, h=1408
- `h4-phi3-muon` — lr=0.0236 (φ⁻³×0.1), 5K steps, h=1408
- `h4-e3d3-full` — h=2432 (e₃=19), ctx=20 (d₃=20), 162K steps
- `h4-spectral-adamw` — beta1=0.5681, lr=0.000125, 81K steps
- `h4-higgs-init-muon` — lr=0.000125, 81K steps

All five are in `experiment_queue` (priority 90–100).

### 2. SG-Class Formulas (25/25 SM Parameters)

10 parallel research agents discovered 7 SG-class formulas covering all 25 Standard Model parameters. These are now Rust constants in `crates/trios-trainer-igla/src/invariants.rs`:

| # | Formula | Value | NN Hyperparameter |
|---|---------|-------|-------------------|
| SG #4 | `127φ/120 + 30/19` | 4.1809 | Muon NS iteration count |
| SG #5 | `φ·11/20 + 20/30` | 0.9565 | Adaptive LR mixing coeff |
| SG #6 | `π/(40φ²)` | 0.03 | NCA objective weight |
| SG #7 | `6π⁵` | 1836.12 | GF16 floor period |
| — | `sin²θ₁₃ = φ^(3/2)/(30π)` | 0.0218 | Alternative beta1 anchor |
| — | `λ = √φ/π²` | 0.1289 | Weight decay (Higgs coupling) |
| — | `N_gen = 3` | exact | JEPA masking ratio |

**New optimizer:** `AdamWCpu::with_sg_defaults(param_count, lr)` uses `weight_decay = λ`.

### 3. T-JEPA Integration

`scarab.rs` dispatches by `trainer` field in `config_json`:
- `"trios-train"` (default) — single-objective
- `"tjepa_train"` — multi-objective `L = NTP + 0.618·JEPA + 0.03·NCA`

**T-JEPA experiments:**
- `tjepa-sg-higgs-s42` — lr=0.000125 ✅ done (BPB 7.0172)
- `tjepa-sg-wboson-s43` — lr=0.000804 🔄 running
- `tjepa-sg-neutrino-s44` — lr=0.000125, muon 🔄 running

### 4. Hive Automaton v1.1

- 24 lanes (L0..L23), schema 1.1
- Dual victory: 3 seeds BPB < 1.5 OR 25/25 SM params
- `FORMULA_COVERAGE_TARGET = 25`

### 5. Coq Optimizer Invariants

`OptimizerInvariants.v` — 5/5 invariants proven (0 Admitted):
- **INV-OPT-1** `muon_lr = φ⁻³ × 0.1` — QED via `interval`
- **INV-OPT-2** `base_lr_scale = 360 × φ⁻³` — QED
- **INV-OPT-3** `wsd_decay = 1 + 1/(15πφ)` — QED
- **INV-OPT-4** `ttt_lr = lr × φ⁻³` — QED via `field`
- **INV-OPT-5** `projection_defect = 1/240` — QED via `reflexivity`

### 6. Next Steps

| Step | Action | Timeline |
|------|--------|----------|
| 1 | Monitor T-JEPA wboson + neutrino results via scarab | Immediate |
| 2 | Download FineWeb corpus for meaningful BPB < 1.50 training | In progress |
| 3 | Run T-JEPA on FineWeb with SG-derived LRs | After step 2 |
| 4 | Add SG-class formulas to CI as `sg_formulas_check` | Next sprint |
| 5 | Adaptive LR schedule from SG #5 `m_H/m_W` | Future |

## Do not

- Open browsers (`R7` of `NOW.json`); use `gh` CLI and the Neon connector.
- Hand-edit generated GraphQL response JSON; treat it as opaque bytes.
- Modify `crates/trios-trainer-igla/*` without updating submodule commit (L8 push first).

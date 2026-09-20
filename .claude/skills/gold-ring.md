# GOLD Ring Architecture Skill

Skill for working with the GOLD ring crate architecture in the trios ecosystem.

## Overview

**EPIC:** gHashTag/trios#446 — WAVE-GF-001  
**Goal:** Convert monolith `crates/trios-igla-race/` (~250 KB) into three GOLD ring crates with strict SR ring decomposition.

## GOLD Crate Structure

```
crates/trios-igla-race-pipeline/          # GOLD I
├── Cargo.toml
├── README.md · TASK.md · AGENTS.md       # I5 mandatory
├── RING.md                               # GOLD ring spec
├── src/lib.rs                            # facade only (≤ 50 LoC)
└── rings/
    ├── SR-00/  scarab-types              # serde + uuid + chrono
    ├── SR-01/  strategy-queue            # hive_automaton + pull_queue
    ├── SR-02/  trainer-runner            # sampler + attn + gf16 + git dep
    ├── SR-03/  bpb-writer                # bpb + ema + neon (write side)
    ├── SR-04/  gardener                  # asha + invariants + race + nca
    ├── SR-05/  railway-deployer          # git dep trios-railway-audit
    └── BR-OUTPUT/                        # IglaRacePipeline assembler + tests

crates/trios-algorithm-arena/             # GOLD II
└── rings/
    ├── SR-ALG-00/  arena-types
    ├── SR-ALG-01/  jepa
    ├── SR-ALG-02/  universal-transformer
    ├── SR-ALG-03/  e2e-ttt              # ★ WIN vs PR #1837
    ├── SR-ALG-04/  phi-nta
    ├── SR-ALG-05/  ssm
    ├── SR-ALG-06/  megakernels
    └── BR-OUTPUT/                        # AlgorithmArena assembler

crates/trios-igla-race-hack/              # GOLD III
└── rings/
    ├── SR-HACK-00/  glossary
    ├── SR-HACK-01/  pr-bot
    ├── SR-HACK-02/  leaderboard-mirror
    ├── SR-HACK-03/  invite
    ├── SR-HACK-04/  equal-billing-coq
    ├── SR-HACK-05/  discord-bridge
    └── BR-OUTPUT/
```

## Dependency Flow (R1 Isolation)

**Rule:** `BR-OUTPUT → SR-05 → SR-04 → SR-03 → SR-02 → SR-01 → SR-00`

- Strict monotone numbering
- NO sibling imports
- NO upward imports
- `SR-00` deps limited to: `serde`, `serde_json`, `uuid`, `chrono`

## Facade Rule (L-ARCH-001)

Every GOLD `src/lib.rs` MUST be:
- ≤ 50 LoC
- Contains ONLY `pub use` re-exports
- NO business logic

Violation = `trios-doctor` error `R-RING-FACADE-001`.

## Ring Invariants (from AGENTS.md)

- **I5** — every ring carries `README.md`, `TASK.md`, `AGENTS.md` (all three)
- **I-SCOPE** — agents may only modify files inside their assigned crate
- **Numbering** — `XX-00` = identity/core types; `XX-01..N` = monotone layers
- **BR-OUTPUT/APP/BIN/MODEL** = final artifact ring

## PHI LOOP (per LAWS.md §7)

Every sub-issue follows 11 steps:

1. **CLAIM** — sub-issue opened with acceptance criteria
2. **SPEC** — `TASK.md` filled inside the new ring
3. **SEAL** — `TASK.md` locked
4. **HASH** — `SHA256(TASK.md)` stored under `.trinity/state/`
5. **GEN** — code + tests written
6. **TEST** — `cargo test --all` passes
7. **VERDICT** — every acceptance bullet verified
8. **EXPERIENCE** — `.trinity/experience/` updated
9. **SKILL** — `.claude/skills/` updated if workflow changed
10. **COMMIT** — includes `Closes #N` and `Agent: <CODENAME>`
11. **PUSH** — branch pushed, PR opened, CI green, merged

## E2E TTT O(1) Loop Sketch

```rust
impl IglaRacePipeline {
    pub async fn run_e2e_ttt_o1(&mut self) -> Result<VictoryReport, PipelineErr> {
        loop {
            let job = self.queue.claim_one().await?;            // O(1)
            for chunk in self.runner.stream_chunks(&job)? {     // O(1) per chunk
                let bpb = self.runner.step_one_chunk(chunk)?;   // O(1) — E2E TTT
                self.writer.write_one(&job, &bpb).await?;       // O(1)
                if self.gardener.should_prune(&job, &bpb).await? { break; }
            }
            if self.gardener.is_victory(&job).await? {           // INV-7
                return Ok(self.gardener.victory_report(&job).await?);
            }
        }
    }
}
```

## 12 Sub-Issues Decomposition

| # | Sub-issue | Risk | Days | Priority | Kingdom |
|---|-----------|------|------|----------|---------|
| 1 | SR-HACK-00 glossary | 🟢 zero | 1 | P2 | Structure |
| 2 | SR-00 scarab-types | 🟢 zero | 1 | P1 | Rust |
| 3 | SR-03 bpb-writer | 🟢 low | 2 | P1 | Rust |
| 4 | SR-01 strategy-queue | 🟢 low | 2 | P1 | Rust |
| 5 | SR-02 trainer-runner | 🟡 medium | 3 | P1 | Cross-kingdom |
| 6 | SR-04 gardener | 🟡 medium | 3 | P1 | Rust |
| 7 | SR-ALG-00 arena-types | 🟢 zero | 1 | P2 | Rust |
| 8 | SR-ALG-03 e2e-ttt (★ WIN) | 🟡 medium | 3 | P0 | Cross-kingdom |
| 9 | SR-05 railway-deployer | 🟡 medium | 2 | P2 | Network |
| 10 | BR-OUTPUT IglaRacePipeline | 🟢 low | 1 | P1 | Rust |
| 11 | trios-doctor SILVER-RING-DR-04 | 🟢 zero | 1 | P2 | Lint |
| 12 | GOLD III SR-HACK-01..05 | 🟢 low | 4 | P3 | Cross-kingdom |

**Critical path to WIN:** 1 → 2 → 3 → 7 → 8 → 5 → 10 (≈ 6 days parallel, 11 days single-stream)

## Source-of-Truth Boundaries

- **Trainer / model / optimizer / JEPA / tokenizer / Dockerfile** → `gHashTag/trios-trainer-igla`
  - Consumed as **versioned git dependency** under feature `trios-integration`
  - NEVER re-implemented inside `trios`
- **Runtime invariants INV-1..INV-10, ASHA scheduler, victory gate** → canonical in `gHashTag/trios`
- **Operator surface (Railway GraphQL, audit, tri-railway CLI)** → `gHashTag/trios-railway`
  - SR-05 integrates via versioned git dep

## New trios-doctor Rules (SILVER-RING-DR-04)

| Rule ID | Description | T0 | T+30 |
|---------|-------------|-----|------|
| R-RING-FACADE-001 | GOLD `src/lib.rs` ≤ 50 LoC, `pub use` only | warn | error |
| R-RING-DEP-002 | SR-00 deps limited to `serde`, `serde_json`, `uuid`, `chrono` | warn | error |
| R-RING-FLOW-003 | SR-NN must not import SR-NN+1 | error | error |
| R-RING-BR-004 | Each GOLD crate has at least one BR-OUTPUT | warn | error |
| R-MCP-BRIDGE-005 | Inter-crate A2A through `trios-server` only | error | error |
| R-L1-ECHO-006 | `.sh` check inside ring trees | warn | error |
| R-L6-PURE-007 | `.py` files inside `crates/` forbidden | warn | error |
| R-COQ-LINK-008 | INV-N status sync between Coq and Rust | warn | warn |

## Success Metrics

| Layer | Metric | Target |
|-------|--------|--------|
| Architecture | `lib.rs` line count | ≤ 50 LoC |
| Architecture | Distinct SR rings shipped | 19 (6+7+6) |
| Architecture | trios-doctor errors | 0 |
| Performance | `step_one_chunk` p99 | < 50 ms (CPU) |
| Performance | per-chunk Neon INSERT p99 | < 10 ms |
| Algorithm | E2E TTT 3-seed mean BPB | **< 1.07063** (beat PR #1837) |
| Coq | INV-7 `igla_found_criterion` Qed | before Gate-3 |
| CI | full pipeline integration test | < 60 s |

## Do-Not List

- ❌ Do not modify `LAWS.md` without §8 amendment
- ❌ Do not edit files outside assigned crate (I-SCOPE)
- ❌ Do not redeploy live fleet (observe-only until SR-05 swap approved)
- ❌ Do not put business logic in GOLD `src/lib.rs`
- ❌ Do not bypass `trios-server` for inter-agent traffic
- ❌ Do not place `.py` payloads inside `crates/`
- ❌ Do not commit without `Agent: <CODENAME>` trailer
- ❌ Do not declare done with unpushed commits

## Quick Commands

```bash
# Check ring facade rule
wc -l crates/trios-*/src/lib.rs

# Verify dependency flow (no upward imports)
cargo tree -p trios-igla-race-pipeline

# Run doctor ring checks
cargo run -p trios-doctor -- check

# Check all tests
cargo test --workspace

# Clippy zero warnings
cargo clippy --all-targets --all-features -- -D warnings
```

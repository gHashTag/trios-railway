# SR-HACK-00: Trinity Glossary

Glossary for Trinity ring-pattern architecture — structural definitions for all outreach DMs, PR comments, and internal docs.

## Purpose

This ring provides the foundational terminology for the Trinity system. All terms are:

- **Self-contained** — Each term has a precise definition
- **Well-documented** — Markdown representation for immediate use
- **Type-safe** — Rust enum with serde support for interchange

## Term Taxonomy

The 16 terms fall into three categories:

### Core Architecture (5)
- **PipelineO1** — O(1) Test-Time Training pipeline
- **RingTier** — Ring structural tier (I/II/III)
- **Lane** — Execution lane
- **Gate** — Quality gate
- **AlgorithmEntry** — Algorithm arena entry point

### Knowledge & Constraints (4)
- **KINGDOM** — Knowledge graph domain
- **I-SCOPE** — Inverse scope constraint
- **LAWS** — Constitutional laws
- **DONE** — Completion checklist

### Agent & Execution (7)
- **Agent** — Autonomous executor
- **Codename** — Agent codename
- **Soul-name** — Agent soul name
- **HEARTBEAT** — Status heartbeat
- **SR** — Speculative research
- **BR** — Bug report
- **INV** — Inventory entry

## Usage

```rust
use trios_igla_race_hack::Term;

// Display term
println!("{}", Term::HEARTBEAT); // HEARTBEAT

// Get markdown for docs
let docs = Term::Agent.as_markdown();
println!("{}", docs);
```

## References

- EPIC #446 — Ring-Pattern Refactor
- LAWS.md v2.0 — Agent dispatch system
- Issue #236 — HEARTBEAT format

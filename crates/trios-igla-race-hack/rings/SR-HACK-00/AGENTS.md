# AGENTS.md — SR-HACK-00: Trinity Glossary

## Agent Codename Assignment

This ring is owned by **GAMMA** per the trios agent dispatch system (see EPIC #446).

## Agent Responsibilities

GAMMA agents working on SR-HACK-00 are responsible for:

1. Maintaining the vocabulary type definitions (`Term`, `Lane`, `Gate`, `RingTier`)
2. Ensuring all terms have proper markdown documentation via `as_markdown()`
3. Keeping unit tests green for all enum variants
4. Adding new terms only when approved via the Trinity governance process

## Forbidden Actions

- ❌ Adding new dependencies beyond `serde` and `serde_json`
- ❌ Modifying terms without proper governance approval
- ❌ Breaking changes to existing term enums without version bump
- ❌ Creating `.sh` files (L1 violation)

## Testing Requirements

Before committing any changes, agents must:

1. Run `cargo test -p trios-igla-race-hack` — all tests must pass
2. Run `cargo clippy --all-targets -- -D warnings` — zero warnings
3. Verify JSON roundtrip for all modified enum variants

## Experience Logging

All work on this ring must be logged to `.trinity/experience/trios_<DATE>.trinity` with:

```
loop: GAMMA | ✅DONE | <task description>

evidence:
  ts: <ISO-8601 UTC>
  issue: #<issue_number>
  ring: SR-HACK-00
  files_modified: <list>
```

## Contact

For questions about this ring, consult:
- EPIC #446 (Ring-Pattern Refactor)
- LAWS.md v2.0
- The Trinity Discord #gold-iii-hack channel

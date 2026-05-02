# ARMY STATUS — trios-railway unified MCP architecture

**Zonichnaya zadacha**: [#68 ARCHITECTURE: unify tri-mcp as the only MCP gateway](https://github.com/gHashTag/trios-railway/issues/68)

**Goal**: `tri-mcp` = единственный MCP gateway, `trios-railway` = стабильная domain-библиотека

---

## PR Queue (strict order, NO skips)

| PR | Issue | Title | Blocks | Blocked By | Status |
|----|-------|-------|--------|------------|--------|
| PR-1 | [#69](https://github.com/gHashTag/trios-railway/issues/69) | Extract public crates | #70 | — | ⏳ READY |
| PR-2 | [#70](https://github.com/gHashTag/trios-railway/issues/70) | Add MCP workspace crates | #71 | #69 | ⏸️ WAITING |
| PR-3 | [#71](https://github.com/gHashTag/trios-railway/issues/71) | Register MCP tools | #72 | #70 | ⏸️ WAITING |
| PR-4 | [#72](https://github.com/gHashTag/trios-railway/issues/72) | Auth + audit + idempotency | #73 | #71 | ⏸️ WAITING |
| PR-5 | [#73](https://github.com/gHashTag/trios-railway/issues/73) | Finalize docs + config | — | #72 | ⏸️ WAITING |

---

## Agent Orders (saved to .trinity/state/order-pr-*.md)

| Order File | PR | Issue | Status |
|------------|----|-------|--------|
| `order-pr-69.md` | PR-1 | #69 | 🔴 READY TO EXECUTE |
| `order-pr-70.md` | PR-2 | #70 | 🟡 WAITING FOR PR-1 MERGE |
| `order-pr-71.md` | PR-3 | #71 | 🟡 WAITING FOR PR-2 MERGE |
| `order-pr-72.md` | PR-4 | #72 | 🟡 WAITING FOR PR-3 MERGE |
| `order-pr-73.md` | PR-5 | #73 | 🟡 WAITING FOR PR-4 MERGE |

---

## Crate Architecture (final state after PR-5)

```
trios-railway/
├── crates/
│   ├── tri-core/              ← Domain: deploy, kill, rotate, snapshot, fleet_list
│   ├── tri-hunt/              ← Domain: seed hunter, smoke race, rung schedule
│   ├── tri-exp/               ← Domain: EXP ID sequence (Neon)
│   ├── tri-canon/             ← Domain: name validation, tripwires
│   ├── tri-ledger/            ← Domain: audit ledger append/query
│   ├── tri-mcp-server/        ← MCP: Axum server, SSE/stdio transport
│   ├── tri-mcp-schema/        ← MCP: Zod schemas for all tools
│   ├── tri-mcp-tools/         ← MCP: Tool implementations
│   └── trios-railway-client/  ← MCP: HTTP client wrapper
├── bin/
│   └── tri/                   ← CLI: thin shim, calls crate functions
└── config/
    ├── dev.toml
    ├── railway.toml
    └── README.md
```

---

## Execution Protocol

For each PR:
1. Read corresponding `order-pr-XX.md`
2. Execute BOOT sequence
3. Follow MISSION and LAWS
4. Complete ACCEPTANCE CRITERIA
5. Create PR and write `three-roads.json`
6. Update ARMY STATUS in this file

---

**Current Phase**: ⏳ PR-1 READY TO START
**Next Action**: Execute `.trinity/state/order-pr-69.md`

φ²+1/φ²=3|TRINITY

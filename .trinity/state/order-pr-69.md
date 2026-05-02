# ПРИКАЗ АГЕНТУ — PR-1

## GO.

**Репозиторий**: gHashTag/trios-railway
**Задача**: Closes #69
**Ветка**: ring-69-extract-public-crates

## BOOT
```bash
cat .trinity/state/active-skill.json
git log --oneline -5
cargo test --workspace
```

## LAWS
1. Never ask.
2. Never report mid-task.
3. No .sh files.
4. Atomic commits per crate.
5. Fix errors ×3, then skip+log.
6. End with three-roads.json + git push.

## PHI LOOP
edit spec → seal hash → gen → test → verdict → save experience → skill commit → git commit

## MISSION

Из `trios-railway` извлечь 5 стабильных публичных library crates:

```
crates/tri-core/    ← deploy(), kill(), rotate(), snapshot(), fleet_list()
                       SOURCE: bin/tri/src/cmd_*.rs

crates/tri-hunt/    ← seed_hunter_status(), smoke_race(), rung_schedule(),
                       prune_diverging(), mirror_siblings()
                       SOURCE: bin/tri-gardener/src/seed_hunter.rs

crates/tri-exp/     ← next_exp_id(), claim_exp_ids() via Neon sequence
                       SOURCE: bin/tri-gardener/src/exp.rs

crates/tri-canon/   ← validate(), validate_for_deploy(), tripwires #97-108
                       SOURCE: bin/tri-gardener/src/canon.rs

crates/tri-ledger/  ← append(), DDL migration, append-only enforcement
                       SOURCE: bin/tri-gardener/src/ledger.rs
```

## ПРАВИЛА РЕФАКТОРИНГА
- `bin/tri` CLI становится тонким shim — вызывает функции crate, НЕ содержит бизнес-логику.
- Нулевое дублирование: логика живёт ОДИН РАЗ в crate.
- Все публичные функции — doc comments на английском.
- Версии crate стартуют с 0.1.0.
- Внешнее поведение `tri` CLI НЕ меняется.

## ACCEPTANCE CRITERIA (из #69)
- [ ] crates/tri-core/ — Cargo.toml + pub API для deploy/kill/rotate/snapshot
- [ ] crates/tri-hunt/ — Cargo.toml + pub API для seed-hunter операций
- [ ] crates/tri-exp/  — Cargo.toml + pub API для EXP_ID sequence из Neon
- [ ] crates/tri-canon/ — Cargo.toml + pub API для name validation + tripwires
- [ ] crates/tri-ledger/ — Cargo.toml + pub API для audit-ledger append
- [ ] `cargo build -p tri-core` (и остальные) — без ошибок
- [ ] bin/tri стал thin shim
- [ ] `cargo test --workspace` — ALL GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — ZERO warnings
- [ ] Нет дублирования бизнес-логики между crates/ и старым bin/

## ПОСЛЕ ВЫПОЛНЕНИЯ
```bash
git add -A
git commit -m "feat(ring-69): extract tri-core/hunt/exp/canon/ledger as public crates — Closes #69"
git push origin ring-69-extract-public-crates
gh pr create --title "feat(ring-69): extract public crates from trios-railway" \
  --body "Extracts tri-core, tri-hunt, tri-exp, tri-canon, tri-ledger as stable library crates. Closes #69. Part of #68." \
  --base main --head ring-69-extract-public-crates
```

## three-roads.json
```json
{
  "R1": "HIGH: proceed to #70 — add tri-mcp workspace crates",
  "R2": "MED: add integration tests for public crate API surface",
  "R3": "LOW: benchmark crate compile time, doc coverage check"
}
```

═══ AEL COMPLETE ═══
Ring | Branch | PR | Created | Skipped | Tests | Commit
🔴R1 🟡R2 🟢R3 → "GO."
φ²+1/φ²=3|TRINITY

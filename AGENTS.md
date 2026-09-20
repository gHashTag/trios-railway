# AGENTS.md — trios-railway

Constitution: same as `gHashTag/trios` (`SOUL.md`, `CLAUDE.md`,
`AGENTS.md`, `LAWS.md`, `NOW.json`, eternal issue
[gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)).
Anchor: `phi^2 + phi^-2 = 3`.

## Scope of this repo

`trios-railway` is the **operator surface** for Railway. It does:

- typed Railway GraphQL queries and mutations
- online audit between Railway reality, the Neon `igla_*` ledger,
  and `.trinity/experience/`
- single-binary CLI `tri-railway` (one verb = one subcommand, L20)

It does **NOT**:

- touch trainer/JEPA/INV-* code (that lives in `trios-trainer-igla`)
- modify `.t27`/`.tri` specs (CANON_DE_ZIGFICATION)
- close eternal issue #143 (L10)

## Standing rules (binding)

- **R1** Rust-only.
- **R5** Honest exit codes; CLI never claims success on upstream failure.
- **R7** Every mutation seals an audit triplet:
  `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`
- **R9** Mutations are gated by `igla check <sha>`.
- **L1** No `.sh` files (CI self-checks).
- **L2** Every PR `Closes #N`.
- **L3** Clippy zero warnings.
- **L4** Tests pass; new code carries new tests.
- **L7** Append a line to `.trinity/experience/` for every significant task.
- **L8** Push first.
- **L11** Pick a soul-name (humorous English) before mutation.
- **L21** `.trinity/experience/` is append-only; never truncate.

## Ring layout

```
crates/
├── trios-railway-core/         RW-00 identity types · RW-01 transport
├── trios-railway-audit/        AU-00 DDL · AU-01 drift detector
└── trios-railway-experience/   EX-00 append-only writer
bin/
└── tri-railway/                BR-CLI entry point
```

Each ring carries `README.md` + `TASK.md` + `AGENTS.md` describing its
local invariants (issue
[#11](https://github.com/gHashTag/trios-railway/issues/11)).

## Commit etiquette

```
feat(rw-01): typed list-services query

Closes #4
Agent: GENERAL
```

## Do not

- Touch `crates/trios-trainer-igla/*` — different repo entirely.
- Open browsers (`R7` of `NOW.json`); use `gh` CLI and the Neon connector.
- Hand-edit generated GraphQL response JSON; treat it as opaque bytes.

## Scarab control invariant (ADR-0042)

Scarabs (matrix-runner / trainer / strategy services) are **forever-live**
on Railway. They pull `ssot.scarab_strategy WHERE service_id=$me` every N
seconds and self-restart on row change. Control flows through the
**Queen-Hive MCP writer** as `INSERT`/`UPDATE` against
`ssot.scarab_strategy`, NOT through Railway GraphQL.

Forbidden against the scarab fleet from this repo:

- `variableUpsert` (env mutation)
- `serviceInstanceDeployV2` / `serviceInstanceRedeploy`
- `serviceInstanceUpdate` (image pin — DR template only)
- `serviceDelete` (use SSOT `status='quarantine'` instead)

The Rust write surface in `crates/trios-railway-core` and the
`tri-railway service deploy/redeploy/delete` verbs are gated behind
`LEGACY_PUSH_PATH_ENABLE=1`. CI / cron must never set that variable.
Read-only diagnostics (audit watchdog, fleet snapshot, MCP diagnose)
remain freely available.

`.github/workflows/` invariants (enforced by
`trios-railway-audit::workflows`):

- Every workflow that uses a Railway push mutation is either:
  - **hard-disabled** with a refuse-job (legacy scarab fleet push —
    `[ADR-0042 disabled]` prefix on the workflow `name:`), or
  - **step-gated** by `env.LEGACY_PUSH_PATH_ENABLE == '1'` (only
    `gardener-loop.yml`; cron path stays read-only), or
  - **double-key gated** with input `confirm == 'PHI'` AND repo secret
    `LEGACY_PUSH_PATH_ENABLE == '1'` (operator-tier recovery only:
    `mcp-emergency-redeploy.yml`, `writer-env-fix.yml`,
    `deploy-from-template.yml`, `redeploy-single.yml`).
- No `schedule:`, `push:`, `pull_request:`, `workflow_run:`, or
  `repository_dispatch:` trigger may reach a push-mutation workflow
  (the gardener schedule is the lone exception and only fires
  read-only stages).

Full rationale: [`docs/ADR-0042-pull-loop.md`](docs/ADR-0042-pull-loop.md).

## Own language first

When this project publishes something about itself, it publishes in **this
project's own language and format** -- not translated into somebody else's.

Owner's rule, 2026-09-20: stop writing in other people's languages, we have our
own.

This bites on any file whose only reason to exist is that an outside tool
expects that shape: `llms.txt`, `agents.json`, `ai.txt`, `.well-known/*.json`,
A2A agent cards, `ai-plugin` manifests, OpenAPI stubs, JSON-LD blocks, a README
that restates a spec. The reflex is to write four of them in four foreign
formats, and the reflex is wrong: a project whose claim is "here is a language
worth writing" and which then describes itself in three of other people's
formats has published three documents that are not true of it.

**The move:** find the address the outside world already fetches, then serve our
own language at it. `/llms.txt` at t27.ai **is** a t27 module -- `llms.txt`
requires nothing but text, and every prose line of a `.t27` file is a `;`
comment, so it stays readable to anything that cannot compile it.

**Three qualifications, so the rule stays honest:**

- A format a resolver genuinely parses -- a sitemap, `package.json`, a lockfile
  -- is machinery, not a description. **Generate** it from our own source; never
  hand-write it into a second home for the truth.
- Code against someone else's API uses their types. Prose for a human who has
  never heard of the project uses that human's language.
- If a format demands a claim we cannot back, **publish nothing**. An A2A card
  with no A2A server behind it is a false claim, and a missing file is more
  honest than a lying one.

The test: *is this file the project speaking about itself?* If yes, it speaks
our language. If it is plumbing, it speaks the plumbing's.

**Worked example, compiler-checked rather than asserted:** in `gHashTag/trinity`,
`apps/website/public/t27/files/specs/catalog/onboarding.t27` generates
`/llms.txt` and `/agents.t27` byte-identically, gated in CI as
`check:onboarding`. The generator evaluates the spec's own `test` blocks --
`typecheck.ok` stays true for `assert 1 > 2`, so a compiler saying "this parses"
is not a compiler saying "this is true" -- and re-compiles the rendered document
before writing it.

**The full rule lives in exactly one place: the `own-language-first` skill**
(`~/.claude/skills/own-language-first/SKILL.md`). It carries the consent gate for
documents addressed to other people's agents, the six negative controls, and the
`;`-alone-on-a-line trap that silently discards a `module` declaration. This
section is a pointer, not a copy -- the recorded defect in this codebase family
is the hand-copied rule that only two of its three homes knew about.

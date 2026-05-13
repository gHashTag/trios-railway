# trios-railway-mcp — multi-account operator guide

One MCP instance, four operator accounts, one URL in Perplexity.

Closes [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61) /
[trios-mcp#2](https://github.com/gHashTag/trios-mcp/issues/2).

Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`.

## Why this exists

Before this PR, every `trios-railway-mcp` instance read a single
`RAILWAY_TOKEN` and could only mutate the one Railway account that
token belonged to. The race fleet today spans **four** Railway
accounts:

| Account | Email |
|---|---|
| `acc0` | `kaglerslomaansc@hotmail.com` |
| `acc1` | `rumbodzalaclhdv0@hotmail.com` |
| `acc2` | `brabbtjubindt5cug@hotmail.com` |
| `acc3` | `gondiigamzevup@hotmail.com` |

To control the fleet you needed four MCP instances and four connectors
in Perplexity. This PR adds `trios_railway_core::multiclient` and
threads an `account` argument through every MCP tool so a **single**
instance routes per call to the right account-scoped token.

## Required environment variables

For every account slot you want to expose, set the four-variable block
below. Slots without a token are silently skipped — you can run with
just one account if you only have one personal token.

```bash
# acc0 — kaglerslomaansc@hotmail.com
RAILWAY_TOKEN_ACC0=<personal-API-token-from-railway.com/account/tokens>
RAILWAY_PROJECT_ID_ACC0=265301ce-0bf2-4187-a36f-348b0eb9942f
RAILWAY_ENVIRONMENT_ID_ACC0=f3517e98-c11a-49d8-b5fd-4cbb82d04384
RAILWAY_TOKEN_KIND_ACC0=team        # or `project` for Project-Access-Tokens

# acc1 — rumbodzalaclhdv0@hotmail.com
RAILWAY_TOKEN_ACC1=...
RAILWAY_PROJECT_ID_ACC1=e4fe33bb-3b09-4842-9782-7d2dea1abc9b
RAILWAY_ENVIRONMENT_ID_ACC1=54e293b9-00a9-4102-814d-db151636d96e
RAILWAY_TOKEN_KIND_ACC1=team

# acc2 — brabbtjubindt5cug@hotmail.com
RAILWAY_TOKEN_ACC2=...
RAILWAY_PROJECT_ID_ACC2=39d833c1-4cb6-4af9-b61b-c204b6733a98
RAILWAY_ENVIRONMENT_ID_ACC2=bce42949-d4ab-43d9-89d1-a6fcc576f45a
RAILWAY_TOKEN_KIND_ACC2=team

# acc3 — gondiigamzevup@hotmail.com  (new — fill in once project + env are created)
RAILWAY_TOKEN_ACC3=...
RAILWAY_PROJECT_ID_ACC3=<project-uuid>
RAILWAY_ENVIRONMENT_ID_ACC3=<environment-uuid>
RAILWAY_TOKEN_KIND_ACC3=team

# legacy single-token fallback — kept so existing one-account
# deployments do not break. Tools without an explicit `account` argument
# still hit this token. Recommended: set it to acc0 (or whichever is
# your "default").
RAILWAY_TOKEN=<same-as-RAILWAY_TOKEN_ACC0>
RAILWAY_TOKEN_AUTH=team

# Neon ledger (shared across accounts)
NEON_DATABASE_URL=postgres://...
PORT=8080
RUST_LOG=info
```

`RAILWAY_TOKEN_KIND_ACC{N}`:

- `team` / `personal` / `bearer` → `Authorization: Bearer <token>` (default for personal API tokens from `railway.com/account/tokens`)
- `project` → `Project-Access-Token: <token>` (for project-scoped tokens)
- omitted → auto-detected: UUID-shaped tokens default to `project`, anything else to `team`

## Tool-call examples

Every `railway_service_*` tool accepts an optional `account` parameter.
Omitting it falls back to the legacy `RAILWAY_TOKEN`. Setting it routes
through `RailwayMultiClient::get(<id>)`.

```jsonc
// list services on acc1's IGLA project
{
  "tool": "railway_service_list",
  "arguments": { "account": "acc1" }
}

// deploy a new train_v2 mirror on acc2
{
  "tool": "railway_service_deploy",
  "arguments": {
    "account": "acc2",
    "name": "IGLA-TRAIN_V2-FP32-E0501-MIRROR-rng43",
    "image": "ghcr.io/ghashtag/trios-trainer-igla:train_v2-latest",
    "vars": [
      { "key": "TRIOS_SEED", "value": "43" },
      { "key": "TRIOS_HIDDEN", "value": "1024" }
    ]
  }
}

// cull a cull-pending attention service on acc1 (R9: confirm required)
{
  "tool": "railway_service_delete",
  "arguments": {
    "account": "acc1",
    "service": "a2a24d1c-5b79-402a-a37f-83cee21a65c6",
    "confirm": true
  }
}
```

If a tool call requests an account whose token is **not** registered in
this MCP instance, the response is the typed
`RailwayError::NotAuthorized { account }` wrapped in an `McpError`
with the operator-friendly message:

> `account "acc3" not authorized in this MCP instance: account not authorized: Acc3 (no creds registered). Set RAILWAY_TOKEN_ACC3 (and _PROJECT_ID_/_ENVIRONMENT_ID_/_TOKEN_KIND_).`

Honest passthrough — never silent.

## Deploy this MCP on a control-plane Railway account

The MCP itself runs in **one** Railway service that has all four
operator tokens injected as env vars. Recommended to host it on a
fifth, dedicated control-plane account (or on `acc0` if you want one
fewer login):

```bash
# Pull the public image
docker pull ghcr.io/ghashtag/trios-railway-mcp:latest

# OR build from source (anchored to this PR's branch)
docker build -f Dockerfile.mcp -t trios-railway-mcp:multi-acc .
docker tag trios-railway-mcp:multi-acc ghcr.io/<your-account>/trios-railway-mcp:multi-acc
docker push ghcr.io/<your-account>/trios-railway-mcp:multi-acc
```

Railway dashboard → **New Service → Docker Image →
`ghcr.io/ghashtag/trios-railway-mcp:latest`** → paste the env block
above into Variables → deploy. Healthcheck path: `/healthz`. Listen
port: `8080`.

## Add as a Perplexity custom remote connector

Per the
[Perplexity help-center](https://www.perplexity.ai/help-center/en/articles/13915507-adding-custom-remote-connectors):

1. **Account settings → Connectors → + Custom connector**
2. Choose **Remote**
3. Fill in:
   - **Name:** `trios-mcp-gateway` (or any label)
   - **MCP Server URL:** `https://<your-service>.up.railway.app/mcp`
   - **Transport:** `Streamable HTTP`
   - **Authentication:** `None` (the MCP itself is read-only without
     valid Railway tokens; for extra safety put a bearer header in your
     reverse proxy)
4. Tick the acknowledgement → **Add**
5. New chat → **Sources** → enable `trios-mcp-gateway`

Perplexity will start every tool call with `account: "accN"` per the
schema, and the MCP routes to the right token internally.

## Tests

11 multiclient unit tests in `trios-railway-core`:

| # | Test | Asserts |
|---|---|---|
| 1 | `account_id_string_round_trip` | `from_alias` ↔ `as_str` for all four IDs and several aliases |
| 2 | `register_then_get_returns_client` | round-trip through `register` → `get` |
| 3 | `get_unknown_account_returns_not_authorized` | typed error on missing slot |
| 4 | `debug_format_does_not_leak_token` | `Debug` output redacts `SecretString` |
| 5 | `registered_lists_accounts_in_order` | always sorted `acc0..acc3` |
| 6 | `scope_all_iterates_in_acc0_acc1_acc2_acc3_order` | fleet-fan-out helper |
| 7 | `scope_one_yields_single_account` | singleton helper |
| 8 | `parse_auth_mode_recognises_explicit_kind` | `team`/`project`/`personal`/`bearer` |
| 9 | `parse_auth_mode_falls_back_to_uuid_heuristic` | UUID-like → Project, else Team |
| 10 | `from_env_skips_empty_slots` | partial config is fine |
| 11 | `from_env_picks_up_four_accounts_when_all_set` | full operator config |

Workspace total: **115 / 115 GREEN**.

## Honest scope

- The MCP picks the `RailwayMultiClient` up at every tool call (`from_env()`
  is cheap; no shared state beyond env vars). A future PR can cache it
  once at startup.
- Cross-account fan-out (`Scope::All`) is in `trios-railway-core` but
  not yet wired into a single MCP tool. The natural follow-up is a new
  `railway_fleet_probe` tool that lists services on every registered
  account in one call — keeps PR diff small here.
- Cull / deploy still rely on `RailwayHash::seal` for R7 audit triplets,
  unchanged by this PR.
- This PR does **not** change the legacy single-token path: tools called
  without `account` still hit `RAILWAY_TOKEN`. Existing single-account
  MCP deployments keep working without an env-var change.

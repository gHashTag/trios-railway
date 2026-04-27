# tri-gardener — image build

Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Source of truth

The canonical builder is the GitHub Actions workflow at
[`.github/workflows/gardener-image.yml`](../../.github/workflows/gardener-image.yml).
It pushes to `ghcr.io/ghashtag/tri-gardener` on:

- a tag push matching `v*-gardener` (e.g. `v0.1.0-gardener`)
- a manual `workflow_dispatch` from the Actions tab

Tags produced:

| Trigger | Image tags |
|---|---|
| `workflow_dispatch` | `:<short-sha>` |
| `push` `v*-gardener` | `:<short-sha>`, `:<release-tag>`, `:latest` |

## Auth

The workflow uses `GITHUB_TOKEN` with `packages: write` by default, which is
sufficient for pushes to the **same org's GHCR namespace**
(`ghcr.io/ghashtag/*`).

If you need to push to a different org or you want to test from a fork, add
the secret `GHCR_TOKEN` (a classic PAT with `write:packages`). The workflow
prefers `GHCR_TOKEN` when it exists, falls back to `GITHUB_TOKEN` otherwise.

## Local fallback (no GHCR_TOKEN)

If the workflow is unavailable (e.g. the operator is iterating before
landing this PR, or `GHCR_TOKEN` is missing for cross-org), build & push
locally:

```bash
# 1. Login (once per session)
echo "$GHCR_TOKEN" | docker login ghcr.io -u "$GITHUB_USER" --password-stdin

# 2. Build from the workspace root (Dockerfile expects that context)
docker build \
  --file bin/tri-gardener/Dockerfile \
  --tag ghcr.io/ghashtag/tri-gardener:latest \
  .

# 3. Push
docker push ghcr.io/ghashtag/tri-gardener:latest
```

## Verifying the image

```bash
docker run --rm ghcr.io/ghashtag/tri-gardener:latest tri-gardener --help
docker run --rm ghcr.io/ghashtag/tri-gardener:latest tri-gardener once --review
```

## After pushing

Wire the gardener service via `tri railway service deploy --image=ghcr.io/ghashtag/tri-gardener:latest …` (see [`SKILL.md`](../../skills/user/tri-gardener-runbook/SKILL.md), step 3).

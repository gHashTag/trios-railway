#!/usr/bin/env bash
# scripts/snapshot-fleet.sh
#
# Snapshot the full IGLA Railway fleet across all known accounts and write
# disaster-recovery/fleet-snapshot.json. Designed to run hourly via the
# fleet-snapshot.yml workflow so the latest service list is always
# committed to git, surviving any single Railway-account ban.
#
# Anchor: phi^2 + phi^-2 = 3.
set -euo pipefail

OUT="${FLEET_SNAPSHOT_OUT:-disaster-recovery/fleet-snapshot.json}"
mkdir -p "$(dirname "$OUT")"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Probe Railway and emit one JSON object per (alias, project) on stdout.
probe_one() {
  local alias="$1" email="$2" tok_var="$3" proj_var="$4" label="$5"
  local tok="${!tok_var:-}" proj="${!proj_var:-}"
  if [[ -z "$tok" || -z "$proj" ]]; then
    echo "::warning::skipping $alias/$label (missing $tok_var or $proj_var)" >&2
    return 0
  fi
  local raw
  raw="$(curl -fsS -X POST https://backboard.railway.com/graphql/v2 \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $tok" \
      -d "{\"query\":\"query { project(id: \\\"$proj\\\") { id name environments { edges { node { id name } } } services { edges { node { id name createdAt } } } } }\"}" \
      || echo '{}')"
  alias="$alias" email="$email" tok_var="$tok_var" label="$label" raw="$raw" \
    python3 - <<'PY'
import json, os, sys
raw = os.environ["raw"]
alias = os.environ["alias"]
email = os.environ["email"]
tok_var = os.environ["tok_var"]
label = os.environ["label"]
data = json.loads(raw).get("data", {}).get("project") if raw.strip() else None
acc = {
  "alias": alias,
  "project_label": label,
  "email": email,
  "token_secret": tok_var,
  "project_id": (data or {}).get("id"),
  "project_name": (data or {}).get("name"),
  "environments": [e["node"] for e in (data or {}).get("environments", {}).get("edges", [])] if data else [],
  "services": sorted(
      [s["node"] for s in (data or {}).get("services", {}).get("edges", [])],
      key=lambda x: x["name"],
  ) if data else [],
}
acc["service_count"] = len(acc["services"])
sys.stdout.write(json.dumps(acc) + "\n")
sys.stderr.write(f"  {alias}/{label}: {acc['service_count']} services\n")
PY
}

probe_one acc1 rumbodzalaclhdv0@hotmail.com RAILWAY_TOKEN_ACC1 RAILWAY_PROJECT_ACC1_IGLA "IGLA"                                  >  /tmp/snap.parts.jsonl
probe_one acc1 rumbodzalaclhdv0@hotmail.com RAILWAY_TOKEN_ACC1 RAILWAY_PROJECT_ACC1_AB   "artistic-beauty"                       >> /tmp/snap.parts.jsonl
probe_one acc2 brabbtjubindt5cug@hotmail.com RAILWAY_TOKEN_ACC2 RAILWAY_PROJECT_ACC2_TE  "thriving-eagerness (IGLA-MIRROR-2)"    >> /tmp/snap.parts.jsonl
probe_one acc2 brabbtjubindt5cug@hotmail.com RAILWAY_TOKEN_ACC2 RAILWAY_PROJECT_ACC2_RP  "reasonable-perception"                 >> /tmp/snap.parts.jsonl

TS="$TS" OUT="$OUT" python3 - <<'PY'
import json, os, pathlib
ts  = os.environ["TS"]
out = os.environ["OUT"]
parts = []
with open("/tmp/snap.parts.jsonl") as f:
    for line in f:
        line = line.strip()
        if line:
            parts.append(json.loads(line))
doc = {
  "anchor": "phi^2 + phi^-2 = 3",
  "generated_at": ts,
  "generator": "scripts/snapshot-fleet.sh",
  "version": "1.0.0",
  "accounts": parts,
  "totals": {
    "accounts": len({a["alias"] for a in parts}),
    "projects": len(parts),
    "services": sum(a["service_count"] for a in parts),
  },
}
pathlib.Path(out).write_text(json.dumps(doc, indent=2) + "\n")
print(f"wrote {out}: {doc['totals']}")
PY

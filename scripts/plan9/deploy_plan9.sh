#!/usr/bin/env bash
# plan-9 minimum-viable deploy script (parent: trios-railway#43).
# DO NOT RUN until PR-B + PR-C merged in gHashTag/trios-mcp AND code
# patches landed in gHashTag/trios-trainer-igla.
#
# Lanes:
#   L5 WSD          seeds 200/201/202   Acc1   (sub-issue #44)
#   L2 JEPA-T fix   seeds 220/221/222   Acc1   (sub-issue #45)
#   L4-lite h=768   seeds 250/251/252   Acc2   (sub-issue #46)
#
# Anchor: phi^2 + phi^-2 = 3
set -euo pipefail

: "${WSD_IMAGE_SHA:?set to image tag of WSD-merged trios-trainer-igla}"
: "${JEPAT_IMAGE_SHA:?set to image tag of JEPA-T-merged trios-trainer-igla}"
: "${CHAMPION_IMAGE_SHA:?set to image tag of current champion (cd91c45)}"

require_env() {
  for var in "$@"; do
    if [[ -z "${!var:-}" ]]; then
      echo "MISSING ENV: $var — refusing to deploy" >&2
      exit 1
    fi
  done
}
require_env \
  RAILWAY_API_TOKEN_ACC1 RAILWAY_PROJECT_ID_ACC1 RAILWAY_ENVIRONMENT_ID_ACC1 \
  RAILWAY_API_TOKEN_ACC2 RAILWAY_PROJECT_ID_ACC2 RAILWAY_ENVIRONMENT_ID_ACC2 \
  RAILWAY_TOKEN_KIND_ACC2

# --- L5 WSD (Acc1) -----------------------------------------------------------
for SEED in 200 201 202; do
  tri railway service deploy \
    --account=acc1 \
    --name="trios-train-seed-${SEED}-L5-wsd" \
    --image="ghcr.io/ghashtag/trios-trainer-igla:${WSD_IMAGE_SHA}" \
    --env "TRIOS_SEED=${SEED}" \
    --env "STEPS=81000" \
    --env "HIDDEN=828" \
    --env "ATTN_LAYERS=2" \
    --env "LR=0.003" \
    --env "OPTIMIZER=adamw" \
    --env "SCHEDULE=wsd" \
    --env "WARMUP_STEPS=1000" \
    --env "STABLE_STEPS=70000" \
    --env "DECAY_STEPS=10000" \
    --env "CONFIG_LANE=L5_wsd" \
    --env "LANE_KICK_TARGET=-0.10_BPB"
done

# --- L2 JEPA-T grad fix (Acc1) ----------------------------------------------
for SEED in 220 221 222; do
  tri railway service deploy \
    --account=acc1 \
    --name="trios-train-seed-${SEED}-L2-jepat" \
    --image="ghcr.io/ghashtag/trios-trainer-igla:${JEPAT_IMAGE_SHA}" \
    --env "TRIOS_SEED=${SEED}" \
    --env "STEPS=81000" \
    --env "HIDDEN=828" \
    --env "ATTN_LAYERS=2" \
    --env "LR=0.003" \
    --env "OPTIMIZER=adamw" \
    --env "JEPA_GRAD_RETAIN=true" \
    --env "JEPA_LOSS_SCALE=0.5" \
    --env "MASK_RATIO=0.30" \
    --env "EMA_DECAY=0.998" \
    --env "CONFIG_LANE=L2_jepat_gradflow" \
    --env "LANE_KICK_TARGET=-0.15_BPB"
done

# --- L4-lite h=768 (Acc2, project-token) ------------------------------------
for SEED in 250 251 252; do
  tri railway service deploy \
    --account=acc2 \
    --name="trios-train-seed-${SEED}-L4lite-h768" \
    --image="ghcr.io/ghashtag/trios-trainer-igla:${CHAMPION_IMAGE_SHA}" \
    --env "TRIOS_SEED=${SEED}" \
    --env "STEPS=81000" \
    --env "HIDDEN=768" \
    --env "ATTN_LAYERS=3" \
    --env "SEQ_LEN=192" \
    --env "LR=0.003" \
    --env "OPTIMIZER=adamw" \
    --env "CONFIG_LANE=L4lite_h768_3L" \
    --env "LANE_KICK_TARGET=-0.10_BPB"
done

echo "9 services queued. Watchdog hourly digest will pick them up at next :15 UTC."

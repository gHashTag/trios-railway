#!/bin/bash
# P0: Apply all P0 fixes (prune mocks, NaN guard, replay quorum)
# Total time: ~30 minutes
# Issue: trios-railway#81 (R5-honest - Bomb 1,2,3)

set -e

echo "=========================================="
echo "P0: Apply All Fixes (30 minutes)"
echo "=========================================="
echo ""

# Step 1: Prune mock rows
echo "[P0.a] Pruning 12 mock rows from experiment_queue..."
psql "$NEON_DATABASE_URL" < .trinity/p0_prune_mocks.sql
echo "✓ Mock pruning complete"
echo ""

# Step 2: NaN guard
echo "[P0.b] Adding NaN guard for final_bpb >= 1e10..."
psql "$NEON_DATABASE_URL" < .trinity/p0_nan_guard.sql
echo "✓ NaN guard active"
echo ""

# Step 3: Replay E0058 quorum
echo "[P0.c] Enqueuing E0058 replay on sanctioned seeds (Fibonacci F17-F19)..."
psql "$NEON_DATABASE_URL" < .trinity/p0_replay_e0058_quorum.sql
echo "✓ E0058 replay enqueued (3 experiments, seeds 1597/2584/4181)"
echo ""

echo "=========================================="
echo "P0 Complete: Leaderboard clean + NaN guard active + E0058 replay queued"
echo "=========================================="
echo ""
echo "Next: Monitor E0058 replay experiments for 1K baseline"
echo "Then: P1 Attention backward fix (CRITICAL BLOCKER - 12h deadline)"

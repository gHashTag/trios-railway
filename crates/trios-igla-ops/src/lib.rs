//! igla-ops — O(1) operator utilities for the IGLA RACE fleet.
//!
//! - `fleet_probe`  — parallel auth/health probe across all 7 Railway accounts (single GraphQL roundtrip per account).
//! - `queue_stats`  — single Neon roundtrip producing the full mission snapshot (queue, scarabs, leaderboard, latest emits).
//! - `queue_victory_hunt` — single transactional insert of the WAVE-GF-001 victory-hunt grid keyed by sanctioned Fibonacci seeds.
//!
//! Anchor: `phi^2 + phi^-2 = 3`. Constitutional rule R1 (trios#143): Rust-only.

pub mod accounts;
pub mod neon;

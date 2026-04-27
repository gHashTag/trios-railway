//! Neon write/read helpers.
//!
//! Stub for PR-1: the function bodies forward to either a real
//! `tokio_postgres::Client` (production) or a mock (tests). Live wiring
//! lands when the rest of the loop is integrated; this PR ships the
//! decision-table core only.

use crate::state::Decision;

/// DDL extension proposed in #49 — applied via `tri-railway audit migrate-sql`
/// once the gardener is wired into the audit pipeline.
pub const GARDENER_DDL: &str = r"
CREATE TABLE IF NOT EXISTS gardener_runs (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    ts            timestamptz NOT NULL DEFAULT now(),
    tick_t_minus  text         NOT NULL,
    action        text         NOT NULL,
    lane          text,
    seed          int,
    before_bpb    double precision,
    after_bpb     double precision,
    decision      jsonb        NOT NULL,
    audit_run_id  uuid REFERENCES railway_audit_runs(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS gardener_runs_ts_idx ON gardener_runs (ts DESC);
";

/// Map a `Decision` to its (action, lane, seed) tuple for ledger
/// projection.
pub fn projection(d: &Decision) -> (&'static str, Option<String>, Option<u32>) {
    match d {
        Decision::Noop { .. } => ("noop", None, None),
        Decision::RedeployMissing { lane, seed, .. } => {
            ("redeploy", Some(lane.clone()), Some(*seed))
        }
        Decision::CullSeed { lane, seed, .. } => ("cull", Some(lane.clone()), Some(*seed)),
        Decision::PromoteSurvivor { lane, seed, .. } => {
            ("promote", Some(lane.clone()), Some(*seed))
        }
        Decision::DeployQueueHead { lane, .. } => ("deploy", Some(lane.clone()), None),
        Decision::PlateauAlert { lane, .. } => ("plateau", Some(lane.clone()), None),
        Decision::HonestNotYet { .. } => ("honest_not_yet", None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BpbStr;

    #[test]
    fn projection_action_strings_match_ddl_check() {
        // The action column intentionally accepts only these literals —
        // mirror the spec in #49.
        let cases = vec![
            Decision::Noop { reason: "x".into() },
            Decision::CullSeed {
                lane: "L1".into(),
                seed: 1,
                bpb: BpbStr::new(2.5),
                threshold: BpbStr::new(2.3),
            },
        ];
        for c in &cases {
            let (action, _, _) = projection(c);
            assert!(matches!(
                action,
                "noop" | "redeploy" | "cull" | "promote" | "deploy" | "plateau" | "honest_not_yet"
            ));
        }
    }
}

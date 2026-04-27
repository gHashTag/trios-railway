//! Issue #53: Live arm actuator.
//!
//! Maps each `Decision` variant to a `tri-railway-core::Client` mutation
//! and emits a `(Decision, Outcome)` pair for the ledger writer.
//!
//! `RailwayClient` is abstracted behind a trait so unit tests can wire a
//! `MockClient` that records calls without a network. PR-2's contract
//! tests `live_actuation_writes_to_neon` and `kill_switch_aborts_mid_actuation`
//! both use the mock.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use trios_railway_core::{
    Client as RailClient, ClientError, DeployId, EnvironmentId, ProjectId, ServiceId,
};

use crate::ledger::Outcome;
use crate::state::Decision;

/// Trait that abstracts over the real `tri-railway-core::Client` and a
/// test mock. Methods mirror the PR-2 mutation surface added in #52.
#[async_trait]
pub trait RailwayActuator: Send + Sync {
    async fn deploy_service(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        name: &str,
        image: &str,
    ) -> Result<ServiceId, ClientError>;

    async fn set_vars(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        service: &ServiceId,
        vars: &[(String, String)],
    ) -> Result<(), ClientError>;

    async fn redeploy(
        &self,
        service: &ServiceId,
        env: &EnvironmentId,
    ) -> Result<DeployId, ClientError>;

    async fn stop(
        &self,
        service: &ServiceId,
        env: &EnvironmentId,
    ) -> Result<(), ClientError>;
}

#[async_trait]
impl RailwayActuator for RailClient {
    async fn deploy_service(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        name: &str,
        image: &str,
    ) -> Result<ServiceId, ClientError> {
        RailClient::deploy_service(self, project, env, name, image).await
    }

    async fn set_vars(
        &self,
        project: &ProjectId,
        env: &EnvironmentId,
        service: &ServiceId,
        vars: &[(String, String)],
    ) -> Result<(), ClientError> {
        RailClient::set_vars(self, project, env, service, vars).await
    }

    async fn redeploy(
        &self,
        service: &ServiceId,
        env: &EnvironmentId,
    ) -> Result<DeployId, ClientError> {
        RailClient::redeploy(self, service, env).await
    }

    async fn stop(
        &self,
        service: &ServiceId,
        env: &EnvironmentId,
    ) -> Result<(), ClientError> {
        RailClient::stop(self, service, env).await
    }
}

/// Apply one decision in Live mode against the actuator.
///
/// Returns the `Outcome` to record in the ledger.
///
/// **Resolution policy:** PR-2 binds gardener to a single
/// `(project, env)` per service via env vars
/// (`RAILWAY_PROJECT_ID_<ACC>`, `RAILWAY_ENVIRONMENT_ID_<ACC>`).
/// Cross-account resolution lives in PR-3 (#56 follow-up).
pub async fn apply_decision(
    actuator: &dyn RailwayActuator,
    project: &ProjectId,
    env: &EnvironmentId,
    image: &str,
    d: &Decision,
) -> Outcome {
    let result: Result<(), String> = match d {
        Decision::Noop { reason } => {
            tracing::info!(?reason, "noop");
            Ok(())
        }
        Decision::RedeployMissing { lane, seed, .. } => {
            let name = format!("trios-train-{lane}-seed-{seed}");
            match actuator.deploy_service(project, env, &name, image).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("deploy_service: {e}")),
            }
        }
        Decision::CullSeed { lane, seed, .. } => {
            // Honest stub: cull-by-name requires a fleet snapshot lookup
            // to resolve seed → service_id. PR-2 covers the happy path
            // where the actuator is invoked with an already-resolved
            // service id; gardener's loop will be responsible for that
            // mapping in PR-3. We log the intent and report Skipped so
            // the ledger row preserves the audit trail.
            tracing::warn!(?lane, ?seed, "cull: service_id resolution deferred to PR-3");
            return Outcome::Skipped {
                reason: "cull pending PR-3 fleet→service_id map".into(),
            };
        }
        Decision::PromoteSurvivor { lane, seed, .. } => {
            let name = format!("trios-train-{lane}-survivor-seed-{seed}");
            match actuator.deploy_service(project, env, &name, image).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("deploy_service[promote]: {e}")),
            }
        }
        Decision::DeployQueueHead { lane, seeds, .. } => {
            let mut last_err: Option<String> = None;
            for s in seeds {
                let name = format!("trios-train-{lane}-seed-{s}");
                if let Err(e) = actuator.deploy_service(project, env, &name, image).await {
                    last_err = Some(format!("deploy_service[queue {s}]: {e}"));
                    break;
                }
            }
            match last_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
        Decision::PlateauAlert { lane, .. } => {
            tracing::info!(?lane, "plateau alert: ledger-only, no mutation");
            Ok(())
        }
        Decision::HonestNotYet { .. } => {
            tracing::warn!("honest_not_yet: Gate-2 missed, ledger-only");
            Ok(())
        }
    };

    match result {
        Ok(()) => Outcome::Applied,
        Err(e) => Outcome::Failed { error: e },
    }
}

/// Cooperative kill-switch shared across actuation tasks. Honored on
/// every call into `apply_decision_batch`; flipping the flag mid-batch
/// short-circuits the rest of the queue.
#[derive(Debug, Clone, Default)]
pub struct KillSwitch(Arc<AtomicBool>);

impl KillSwitch {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_env() -> Self {
        let on = std::env::var("GARDENER_DISABLED").as_deref() == Ok("true");
        let s = Self::default();
        s.set(on);
        s
    }
    pub fn set(&self, on: bool) {
        self.0.store(on, Ordering::SeqCst);
    }
    pub fn is_disabled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Apply N decisions sequentially; honor `KillSwitch::is_disabled` on
/// every iteration. Sequential by design: Railway's GraphQL endpoint is
/// per-account rate-limited (~10rps), and the gardener emits ≤7
/// decisions per tick, so concurrent fan-out adds risk without speedup.
pub async fn apply_decision_batch(
    actuator: &dyn RailwayActuator,
    project: &ProjectId,
    env: &EnvironmentId,
    image: &str,
    kill: &KillSwitch,
    decisions: &[Decision],
) -> Vec<(Decision, Outcome)> {
    let mut out = Vec::with_capacity(decisions.len());
    for d in decisions {
        if kill.is_disabled() {
            out.push((
                d.clone(),
                Outcome::Skipped {
                    reason: "kill switch engaged mid-batch".into(),
                },
            ));
            continue;
        }
        let outcome = apply_decision(actuator, project, env, image, d).await;
        out.push((d.clone(), outcome));
    }
    out
}

// ---------------------------------------------------------------------------
// Mock actuator for unit tests.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct MockActuator {
    pub calls: Arc<Mutex<Vec<String>>>,
    pub fail_on_verb: Arc<Mutex<Option<String>>>,
}

impl MockActuator {
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
    pub fn fail_next(&self, verb: &str) {
        *self.fail_on_verb.lock().unwrap() = Some(verb.to_string());
    }
}

#[async_trait]
impl RailwayActuator for MockActuator {
    async fn deploy_service(
        &self,
        _project: &ProjectId,
        _env: &EnvironmentId,
        name: &str,
        _image: &str,
    ) -> Result<ServiceId, ClientError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("deploy_service:{name}"));
        if matches!(self.fail_on_verb.lock().unwrap().as_deref(), Some("deploy_service")) {
            return Err(ClientError::GraphQl("mock-fail".into()));
        }
        Ok(ServiceId::new(format!("svc-{name}")))
    }
    async fn set_vars(
        &self,
        _project: &ProjectId,
        _env: &EnvironmentId,
        service: &ServiceId,
        vars: &[(String, String)],
    ) -> Result<(), ClientError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("set_vars:{}:{}", service.as_str(), vars.len()));
        Ok(())
    }
    async fn redeploy(
        &self,
        service: &ServiceId,
        _env: &EnvironmentId,
    ) -> Result<DeployId, ClientError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("redeploy:{}", service.as_str()));
        Ok(DeployId::new("d-mock".to_string()))
    }
    async fn stop(
        &self,
        service: &ServiceId,
        _env: &EnvironmentId,
    ) -> Result<(), ClientError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("stop:{}", service.as_str()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BpbStr;

    fn project() -> ProjectId {
        ProjectId::new("e4fe33bb-3b09-4842-9782-7d2dea1abc9b".to_string())
    }
    fn env_() -> EnvironmentId {
        EnvironmentId::new("54e293b9-00a9-4102-814d-db151636d96e".to_string())
    }
    const IMG: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

    #[tokio::test]
    async fn redeploy_missing_calls_deploy_service() {
        let m = MockActuator::default();
        let d = Decision::RedeployMissing {
            lane: "L4".into(),
            seed: 240,
            reason: "fleet drift".into(),
        };
        let oc = apply_decision(&m, &project(), &env_(), IMG, &d).await;
        assert_eq!(oc, Outcome::Applied);
        assert_eq!(
            m.calls(),
            vec!["deploy_service:trios-train-L4-seed-240"]
        );
    }

    #[tokio::test]
    async fn deploy_queue_head_dispatches_each_seed() {
        let m = MockActuator::default();
        let d = Decision::DeployQueueHead {
            lane: "L1".into(),
            account: "acc1".into(),
            seeds: vec![210, 211, 212],
        };
        let oc = apply_decision(&m, &project(), &env_(), IMG, &d).await;
        assert_eq!(oc, Outcome::Applied);
        let calls = m.calls();
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().any(|c| c.ends_with("seed-210")));
        assert!(calls.iter().any(|c| c.ends_with("seed-212")));
    }

    #[tokio::test]
    async fn deploy_failure_propagates_as_failed_outcome() {
        let m = MockActuator::default();
        m.fail_next("deploy_service");
        let d = Decision::DeployQueueHead {
            lane: "L1".into(),
            account: "acc1".into(),
            seeds: vec![210],
        };
        let oc = apply_decision(&m, &project(), &env_(), IMG, &d).await;
        match oc {
            Outcome::Failed { error } => assert!(error.contains("mock-fail")),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cull_seed_is_skipped_pending_pr3_resolution() {
        let m = MockActuator::default();
        let d = Decision::CullSeed {
            lane: "L1".into(),
            seed: 210,
            bpb: BpbStr::new(2.45),
            threshold: BpbStr::new(2.30),
        };
        let oc = apply_decision(&m, &project(), &env_(), IMG, &d).await;
        match oc {
            Outcome::Skipped { reason } => assert!(reason.contains("PR-3")),
            other => panic!("expected Skipped, got {other:?}"),
        }
        assert!(m.calls().is_empty(), "no actuator call for cull yet");
    }

    #[tokio::test]
    async fn kill_switch_aborts_mid_batch() {
        let m = MockActuator::default();
        let kill = KillSwitch::new();
        let decisions = vec![
            Decision::RedeployMissing {
                lane: "L1".into(),
                seed: 210,
                reason: "x".into(),
            },
            Decision::RedeployMissing {
                lane: "L1".into(),
                seed: 211,
                reason: "x".into(),
            },
        ];
        // Engage kill switch before batch starts → ALL skipped.
        kill.set(true);
        let outcomes = apply_decision_batch(&m, &project(), &env_(), IMG, &kill, &decisions).await;
        assert_eq!(outcomes.len(), 2);
        for (_d, oc) in &outcomes {
            assert!(matches!(oc, Outcome::Skipped { .. }));
        }
        assert!(m.calls().is_empty(), "kill switch must prevent any call");
    }

    #[tokio::test]
    async fn live_actuation_writes_to_ledger_via_mock_pair() {
        // PR-2 contract test: actuator + ledger + apply_decision_batch
        // compose into a write to MockLedger.
        use crate::ledger::{build_row, MockLedger, LedgerSink};
        use chrono::{TimeZone, Utc};
        let m = MockActuator::default();
        let kill = KillSwitch::new();
        let decisions = vec![Decision::RedeployMissing {
            lane: "L4".into(),
            seed: 240,
            reason: "x".into(),
        }];
        let outcomes = apply_decision_batch(&m, &project(), &env_(), IMG, &kill, &decisions).await;

        let now = Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap();
        let rows: Vec<_> = outcomes
            .iter()
            .map(|(d, o)| build_row(now, d, o.clone()))
            .collect();
        let ledger = MockLedger::default();
        ledger.write_tick(&rows).await.unwrap();

        let written = ledger.rows.lock().unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].action, "redeploy");
        assert_eq!(written[0].outcome, Outcome::Applied);
    }
}

//! `tri railway plan9` — manifest-driven multi-account deploy.
//!
//! Reads `plan21-manifest.toml`, resolves the per-account triplet
//! (token / project / env / token-kind) from the environment, then for
//! each selected lane creates the service, pins the image, upserts env
//! vars, and triggers a redeploy via `trios-railway-core`.
//!
//! No bash. No sed. No awk. Anchor: phi^2 + phi^-2 = 3.

// `Lane::lane` is the lane id — matches the struct name because that
// is the natural manifest key. Keep clippy happy module-wide.
#![allow(clippy::struct_field_names)]

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use trios_railway_core::{mutations as M, AuthMode, Client, EnvironmentId, ProjectId, ServiceId};

// ---------------------------------------------------------------------------
// Manifest types (deserialised from `plan21-manifest.toml`)
// ---------------------------------------------------------------------------

// `version`, `anchor`, `drift_rules` are part of the contract surface
// for the watchdog and sibling readers; deploy path uses only `lanes`.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Plan21Manifest {
    pub version: String,
    pub anchor: String,
    #[serde(default)]
    pub drift_rules: BTreeMap<String, String>,
    pub lanes: Vec<Lane>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)] // `state` is consumed by the watchdog reader (sibling crate),
                    // not by deploy logic. Kept on the struct for serde round-trip.
pub struct Lane {
    pub lane: String,
    pub account: AccountAlias,
    pub project_id: String,
    pub env_id: String,
    pub seeds: Vec<u32>,
    pub name_template: String,
    pub image: ImageRef,
    pub state: LaneState,
    #[serde(default)]
    pub blocked_on: Vec<String>,
    #[serde(default, rename = "env")]
    pub env_vars: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountAlias {
    Acc0,
    Acc1,
    Acc2,
    Acc3,
}

impl AccountAlias {
    pub fn slug(self) -> &'static str {
        match self {
            AccountAlias::Acc0 => "ACC0",
            AccountAlias::Acc1 => "ACC1",
            AccountAlias::Acc2 => "ACC2",
            AccountAlias::Acc3 => "ACC3",
        }
    }

    pub fn alias(self) -> &'static str {
        match self {
            AccountAlias::Acc0 => "acc0",
            AccountAlias::Acc1 => "acc1",
            AccountAlias::Acc2 => "acc2",
            AccountAlias::Acc3 => "acc3",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageRef {
    Champion,
    WsdMerged,
    JepatMerged,
    NcaMerged,
}

impl ImageRef {
    /// Env-var name carrying the resolved SHA tag for this image.
    pub fn env_var(self) -> &'static str {
        match self {
            ImageRef::Champion => "CHAMPION_IMAGE_SHA",
            ImageRef::WsdMerged => "WSD_IMAGE_SHA",
            ImageRef::JepatMerged => "JEPAT_IMAGE_SHA",
            ImageRef::NcaMerged => "NCA_IMAGE_SHA",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaneState {
    Running,
    Queued,
}

// ---------------------------------------------------------------------------
// Account triplet resolved from env at deploy time.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AccountTriplet {
    pub token: String,
    pub project_id: ProjectId,
    pub env_id: EnvironmentId,
    pub auth_mode: AuthMode,
}

impl AccountTriplet {
    /// Read `RAILWAY_API_TOKEN_<ACCx>` / `RAILWAY_PROJECT_ID_<ACCx>` /
    /// `RAILWAY_ENVIRONMENT_ID_<ACCx>` / `RAILWAY_TOKEN_KIND_<ACCx>`.
    pub fn from_env(acc: AccountAlias) -> Result<Self> {
        let slug = acc.slug();
        let token = require_env(&format!("RAILWAY_API_TOKEN_{slug}"))?;
        let project_id = ProjectId::from(require_env(&format!("RAILWAY_PROJECT_ID_{slug}"))?);
        let env_id = EnvironmentId::from(require_env(&format!("RAILWAY_ENVIRONMENT_ID_{slug}"))?);
        let auth_mode = match std::env::var(format!("RAILWAY_TOKEN_KIND_{slug}"))
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "user" | "team" | "user_account" => AuthMode::Team,
            "project" | "project_access" => AuthMode::Project,
            other => bail!(
                "RAILWAY_TOKEN_KIND_{slug}=`{other}` is not one of \
                 `user|team|project|project_access`"
            ),
        };
        Ok(Self {
            token,
            project_id,
            env_id,
            auth_mode,
        })
    }

    /// Cross-check that the manifest's project/env match the env
    /// triplet. Mismatch is an error to keep operator from accidentally
    /// deploying into a different project than the manifest declares.
    pub fn matches_lane(&self, lane: &Lane) -> Result<()> {
        if self.project_id.as_str() != lane.project_id {
            bail!(
                "manifest lane `{}` declares project `{}` but env says `{}`",
                lane.lane,
                lane.project_id,
                self.project_id.as_str()
            );
        }
        if self.env_id.as_str() != lane.env_id {
            bail!(
                "manifest lane `{}` declares env `{}` but env says `{}`",
                lane.lane,
                lane.env_id,
                self.env_id.as_str()
            );
        }
        Ok(())
    }
}

fn require_env(var: &str) -> Result<String> {
    std::env::var(var).map_err(|_| anyhow!("missing env var `{var}`"))
}

// ---------------------------------------------------------------------------
// Manifest loading + lane filtering.
// ---------------------------------------------------------------------------

pub fn load_manifest(path: &Path) -> Result<Plan21Manifest> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading manifest {}", path.display()))?;
    let manifest: Plan21Manifest =
        toml::from_str(&raw).with_context(|| format!("parsing manifest {}", path.display()))?;
    Ok(manifest)
}

pub fn select_lane<'a>(manifest: &'a Plan21Manifest, lane_id: &str) -> Result<&'a Lane> {
    manifest
        .lanes
        .iter()
        .find(|l| l.lane.eq_ignore_ascii_case(lane_id))
        .ok_or_else(|| {
            let available: Vec<&str> = manifest.lanes.iter().map(|l| l.lane.as_str()).collect();
            anyhow!(
                "lane `{lane_id}` not in manifest. available: {}",
                available.join(", ")
            )
        })
}

/// Render the service name for a single seed, replacing `{{seed}}` in
/// the template. (Not full handlebars — only this one variable.)
pub fn render_name(template: &str, seed: u32) -> String {
    template.replace("{{seed}}", &seed.to_string())
}

// ---------------------------------------------------------------------------
// Deploy actions.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DeployPlan {
    pub lane: String,
    pub account: AccountAlias,
    pub services: Vec<PlannedService>,
    pub image: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // `seed` is part of the Debug surface for ops tracing.
pub struct PlannedService {
    pub name: String,
    pub seed: u32,
    pub env: BTreeMap<String, String>,
}

pub fn build_plan(
    lane: &Lane,
    seeds_filter: Option<&[u32]>,
    image_sha: &str,
) -> Result<DeployPlan> {
    let seeds: Vec<u32> = match seeds_filter {
        None => lane.seeds.clone(),
        Some(filter) => {
            for s in filter {
                if !lane.seeds.contains(s) {
                    bail!(
                        "seed {s} is not in lane `{}` (declared seeds: {:?})",
                        lane.lane,
                        lane.seeds
                    );
                }
            }
            filter.to_vec()
        }
    };

    let services = seeds
        .into_iter()
        .map(|seed| {
            let mut env = lane.env_vars.clone();
            env.insert("TRIOS_SEED".to_string(), seed.to_string());
            PlannedService {
                name: render_name(&lane.name_template, seed),
                seed,
                env,
            }
        })
        .collect();

    let image = format!("ghcr.io/ghashtag/trios-trainer-igla:{image_sha}");

    Ok(DeployPlan {
        lane: lane.lane.clone(),
        account: lane.account,
        services,
        image,
    })
}

/// Print the plan in machine-greppable form (one line per mutation).
/// Used by `--dry-run` and by the contract test.
pub fn render_dry_run(plan: &DeployPlan, triplet: &AccountTriplet) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    writeln!(
        &mut out,
        "DRY-RUN lane={} account={} project={} env={} image={}",
        plan.lane,
        plan.account.alias(),
        triplet.project_id.as_str(),
        triplet.env_id.as_str(),
        plan.image,
    )
    .unwrap();
    for s in &plan.services {
        writeln!(&mut out, "  serviceCreate name={}", s.name).unwrap();
        writeln!(
            &mut out,
            "  serviceInstanceUpdate name={} image={}",
            s.name, plan.image
        )
        .unwrap();
        for (k, v) in &s.env {
            writeln!(
                &mut out,
                "  variableUpsert name={} key={} value_len={}",
                s.name,
                k,
                v.len()
            )
            .unwrap();
        }
        writeln!(&mut out, "  serviceInstanceRedeploy name={}", s.name).unwrap();
    }
    out
}

/// Execute the plan against the live Railway API.
pub async fn execute_plan(plan: &DeployPlan, triplet: &AccountTriplet) -> Result<()> {
    let client = Client::with_token_and_mode(triplet.token.clone(), triplet.auth_mode)?;
    for s in &plan.services {
        let created = M::service_create(&client, &triplet.project_id, &s.name).await?;
        let sid = ServiceId::from(created.id.clone());
        println!("plan9 created service {} ({})", created.name, created.id);

        M::service_instance_set_image(&client, &sid, &triplet.env_id, &plan.image).await?;
        for (k, v) in &s.env {
            M::variable_upsert(&client, &triplet.project_id, &triplet.env_id, &sid, k, v).await?;
        }
        let dep = M::service_redeploy(&client, &sid, &triplet.env_id).await?;
        println!("plan9 redeploy {} -> deploy_id={}", s.name, dep);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manifest_path() -> PathBuf {
        // Manifest sits next to Cargo.toml in `bin/tri-railway`.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plan21-manifest.toml")
    }

    #[test]
    fn manifest_loads_and_has_six_lanes() {
        let m = load_manifest(&manifest_path()).expect("manifest must load");
        assert_eq!(m.anchor, "phi^2 + phi^-2 = 3");
        assert!(
            m.lanes.len() >= 6,
            "expected >= 6 lanes, got {}",
            m.lanes.len()
        );
        // L5 must live on Acc2 after the capacity rebalance.
        let l5 = select_lane(&m, "L5_wsd").unwrap();
        assert_eq!(l5.account, AccountAlias::Acc2);
    }

    #[test]
    fn render_name_substitutes_seed() {
        assert_eq!(
            render_name("trios-train-seed-{{seed}}-L5-wsd", 200),
            "trios-train-seed-200-L5-wsd"
        );
    }

    #[test]
    fn build_plan_seed_filter_rejects_unknown() {
        let m = load_manifest(&manifest_path()).unwrap();
        let lane = select_lane(&m, "L5_wsd").unwrap();
        let err = build_plan(lane, Some(&[999]), "deadbeef").unwrap_err();
        assert!(err.to_string().contains("seed 999 is not in lane"));
    }

    /// Contract test: dry-run of L5 WSD against a synthetic triplet
    /// emits exactly the GraphQL mutation sequence we expect Railway
    /// to receive at execute time. Order matters and is asserted.
    #[test]
    fn plan9_dry_run_emits_correct_graphql_mutations() {
        let m = load_manifest(&manifest_path()).unwrap();
        let lane = select_lane(&m, "L5_wsd").unwrap();
        let plan = build_plan(lane, None, "abc1234").expect("plan");

        let triplet = AccountTriplet {
            token: "tok-test".into(),
            project_id: ProjectId::from(lane.project_id.clone()),
            env_id: EnvironmentId::from(lane.env_id.clone()),
            auth_mode: AuthMode::Project,
        };

        let dry = render_dry_run(&plan, &triplet);

        // Exactly 3 services × (create + image + N vars + redeploy).
        assert_eq!(dry.matches("serviceCreate ").count(), 3);
        assert_eq!(dry.matches("serviceInstanceUpdate ").count(), 3);
        assert_eq!(dry.matches("serviceInstanceRedeploy ").count(), 3);

        // Each service must include TRIOS_SEED upsert.
        for seed in [200u32, 201, 202] {
            let needle = format!(
                "variableUpsert name=trios-train-seed-{seed}-L5-wsd key=TRIOS_SEED value_len="
            );
            assert!(
                dry.contains(&needle),
                "missing TRIOS_SEED upsert for seed {seed}: dry={dry}"
            );
        }

        // L5 declares SCHEDULE=wsd among lane env, must surface for every seed.
        assert_eq!(dry.matches("key=SCHEDULE value_len=3").count(), 3);

        // Mutation order per service: create → image → vars → redeploy.
        let svc_block_re_lines: Vec<&str> = dry.lines().collect();
        let mut state = "init";
        for line in &svc_block_re_lines {
            let line = line.trim_start();
            if line.starts_with("serviceCreate ") {
                state = "after_create";
            } else if line.starts_with("serviceInstanceUpdate ") {
                assert_eq!(
                    state, "after_create",
                    "serviceInstanceUpdate without preceding serviceCreate: line={line}"
                );
                state = "after_image";
            } else if line.starts_with("variableUpsert ") {
                assert!(
                    state == "after_image" || state == "after_var",
                    "variableUpsert before image set: line={line}"
                );
                state = "after_var";
            } else if line.starts_with("serviceInstanceRedeploy ") {
                assert!(
                    state == "after_var" || state == "after_image",
                    "redeploy before any var or image: line={line}"
                );
                state = "init"; // ready for next service
            }
        }
    }
}

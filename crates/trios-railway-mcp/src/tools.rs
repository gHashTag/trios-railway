//! MCP tool surface for `trios-railway-mcp`.
//!
//! All tools call `trios-railway-core` directly (no shell-out) and
//! emit an L7 experience line on every successful mutation (R7).

use std::path::PathBuf;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use trios_railway_core::{
    canon::IglaCanon,
    multiclient::{assert_project_allowed, AccountId, RailwayMultiClient},
    mutations as M, queries as Q, transport::Client, EnvironmentId, ProjectId, RailwayHash,
    ServiceId,
};
use trios_railway_experience::{append_line, ExperienceLine};

use crate::{connections, tripwires};

/// `via_mcp` marker passed into `tripwires::t109_no_direct_call` so a
/// rogue caller that bypasses this module is blocked at runtime.
const VIA_MCP: &str = "mcp";

const IGLA_PROJECT_ID: &str = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b";
const IGLA_PROD_ENV_ID: &str = "54e293b9-00a9-4102-814d-db151636d96e";
const DEFAULT_TRAINER_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

// -------- request payload structs --------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListServicesRequest {
    /// Project UUID. Defaults to the IGLA project.
    #[serde(default)]
    pub project: Option<String>,
    /// Account alias — `"acc0"`/`"acc1"`/`"acc2"`/`"acc3"`. When set,
    /// the call routes through `RailwayMultiClient` and uses the
    /// per-account `RAILWAY_TOKEN_ACC{N}` instead of the global token.
    /// When omitted, falls back to legacy `RAILWAY_TOKEN`.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeployRequest {
    /// Service name, e.g. `trios-train-seed-49`.
    pub name: String,
    /// Docker image. Defaults to the canonical IGLA trainer image.
    #[serde(default)]
    pub image: Option<String>,
    /// Project UUID. Defaults to the IGLA project.
    #[serde(default)]
    pub project: Option<String>,
    /// Environment UUID. Defaults to IGLA `production`.
    #[serde(default)]
    pub environment: Option<String>,
    /// Reuse an existing service instead of creating a new one.
    #[serde(default)]
    pub existing_service_id: Option<String>,
    /// Env-var pairs to upsert before redeploy.
    #[serde(default)]
    pub vars: Vec<KeyValue>,
    /// Repo root for the L7 experience log. Defaults to `.`.
    #[serde(default)]
    pub experience_root: Option<String>,
    /// Account alias — `"acc0"`/`"acc1"`/`"acc2"`/`"acc3"`. Routes
    /// through `RailwayMultiClient`. Omit to use the legacy single
    /// `RAILWAY_TOKEN` path.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RedeployRequest {
    /// Service UUID to redeploy.
    pub service: String,
    /// Environment UUID. Defaults to IGLA `production`.
    #[serde(default)]
    pub environment: Option<String>,
    /// Account alias — see `ListServicesRequest::account`.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteRequest {
    /// Service UUID to delete.
    pub service: String,
    /// Must be `true` (R9 safety): the call refuses to proceed otherwise.
    pub confirm: bool,
    /// Account alias — see `ListServicesRequest::account`.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExperienceAppendRequest {
    /// Issue ref like `#20`.
    pub issue: String,
    /// PHI LOOP step (CLAIM, NAME, SPEC, SEAL, GEN, TEST, VERDICT,
    /// EXPERIENCE, REPORT, COMMIT, PUSH).
    pub phi_step: String,
    /// Free-form task summary.
    pub task: String,
    /// Status string. Defaults to `OK`.
    #[serde(default)]
    pub status: Option<String>,
    /// Soul-name (humorous English, L11). Defaults to `RailRangerOne`.
    #[serde(default)]
    pub soul_name: Option<String>,
    /// Agent codename. Defaults to `GENERAL`.
    #[serde(default)]
    pub agent: Option<String>,
    /// Verb for the audit triplet. Defaults to `experience`.
    #[serde(default)]
    pub verb: Option<String>,
    /// Project UUID. Defaults to IGLA.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional service UUID for the triplet.
    #[serde(default)]
    pub service: Option<String>,
    /// Repo root. Defaults to `.`.
    #[serde(default)]
    pub root: Option<String>,
}

// -------- request payloads for the curated `mcp.<domain>.<verb>` surface --------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpDeployRequest {
    /// Idempotency key — caller-supplied unique string per logical deploy
    /// attempt (#114). Replaying the same key in the same process returns
    /// a cached "already done" response instead of re-deploying.
    pub idempotency_key: String,
    #[serde(flatten)]
    pub inner: DeployRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpRedeployRequest {
    /// Idempotency key (#114).
    pub idempotency_key: String,
    #[serde(flatten)]
    pub inner: RedeployRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpDeleteRequest {
    /// Dry-run preview (#113). Defaults to `true`. To actually delete,
    /// caller must pass `dry_run=false` AND `confirm=true`.
    #[serde(default)]
    pub dry_run: Option<bool>,
    #[serde(flatten)]
    pub inner: DeleteRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IglaValidateRequest {
    /// Service name in IGLA canonical form, e.g.
    /// `IGLA-TRAIN_V2-FP32-E0042-WSD-rng42`.
    pub name: String,
    /// Optional hidden width `h` for L-R9 GF16 capacity check.
    #[serde(default)]
    pub capacity_h: Option<u32>,
    /// Optional primary loss kind for L-METRIC enforcement ("bpb",
    /// "mse", etc.). Only required when validating JEPA-T / NCA names.
    #[serde(default)]
    pub primary_loss_kind: Option<String>,
    /// Optional current max EXP_ID observed in Neon. When set,
    /// `validate_for_deploy` enforces tripwires #98/#99/#100.
    #[serde(default)]
    pub current_max_exp_id: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FleetSnapshotRequest {
    /// Account alias to snapshot. Omit for fan-out across all
    /// registered accounts (`acc0..acc3`).
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FleetCleanupRequest {
    /// Account alias (#112). REQUIRED — fleet cleanup is account-scoped.
    pub account: String,
    /// Regex that names of services to KEEP must match. Empty pattern
    /// is rejected (no nuclear delete-all).
    pub keep_pattern: String,
    /// Idempotency key (#114).
    pub idempotency_key: String,
    /// Dry-run preview (#113). Defaults to `true`.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// R9 confirmation. Required to actually delete (combined with
    /// `dry_run=false`).
    #[serde(default)]
    pub confirm: bool,
}

// -------- handler --------

#[derive(Clone)]
pub struct TriosRailwayMcp {
    tool_router: ToolRouter<TriosRailwayMcp>,
}

#[tool_router]
impl TriosRailwayMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all Railway services in the IGLA project (or any other project).")]
    async fn railway_service_list(
        &self,
        Parameters(req): Parameters<ListServicesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = build_client_for(req.account.as_deref())?;
        let project = req.project.unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let pid = ProjectId::from(project.clone());
        let pv = Q::project_view(&client, &pid).await.map_err(internal_err)?;
        let services: Vec<_> = pv
            .services()
            .into_iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "name": s.name,
                    "created_at": s.created_at,
                })
            })
            .collect();
        let body = json!({
            "project_id": pv.id,
            "project_name": pv.name,
            "services": services,
            "count": services.len(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(
        description = "Create (or reuse) a Railway service, pin its image, upsert env vars, and trigger a redeploy. Emits an L7 experience line. Requires RAILWAY_TOKEN env var."
    )]
    async fn railway_service_deploy(
        &self,
        Parameters(req): Parameters<DeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = build_client_for(req.account.as_deref())?;
        let token_fp = client.token_fingerprint();

        let project = req.project.unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let environment = req
            .environment
            .unwrap_or_else(|| IGLA_PROD_ENV_ID.to_string());
        let image = req
            .image
            .unwrap_or_else(|| DEFAULT_TRAINER_IMAGE.to_string());

        let pid = ProjectId::from(project);
        let eid = EnvironmentId::from(environment);

        let service_id: ServiceId = if let Some(sid) = req.existing_service_id {
            ServiceId::from(sid)
        } else {
            let created = M::service_create(&client, &pid, &req.name)
                .await
                .map_err(internal_err)?;
            ServiceId::from(created.id)
        };

        M::service_instance_set_image(&client, &service_id, &eid, &image)
            .await
            .map_err(internal_err)?;

        for kv in &req.vars {
            M::variable_upsert(&client, &pid, &eid, &service_id, &kv.key, &kv.value)
                .await
                .map_err(internal_err)?;
        }

        let deploy_id = M::service_redeploy(&client, &service_id, &eid)
            .await
            .map_err(internal_err)?;

        // R7 triplet to local experience log.
        let hash = RailwayHash::seal("deploy", &pid, Some(&service_id), &token_fp);
        let line = ExperienceLine::from_hash(
            "GENERAL",
            "RailRangerOne",
            "#20",
            &format!("mcp deploy {} image={}", req.name, image),
            "OK",
            "PUSH",
            &hash,
        )
        .map_err(internal_err)?;
        let root: PathBuf = req
            .experience_root
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        let path = append_line(&root.join(".trinity"), &line)
            .await
            .map_err(internal_err)?;

        let body = json!({
            "service_id": service_id.as_str(),
            "deploy_id": deploy_id.as_str(),
            "image": image,
            "experience_path": path.display().to_string(),
            "triplet": hash.triplet(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(description = "Trigger a redeploy on an existing Railway service.")]
    async fn railway_service_redeploy(
        &self,
        Parameters(req): Parameters<RedeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = build_client_for(req.account.as_deref())?;
        let env = req
            .environment
            .unwrap_or_else(|| IGLA_PROD_ENV_ID.to_string());
        let sid = ServiceId::from(req.service);
        let eid = EnvironmentId::from(env);
        let deploy_id = M::service_redeploy(&client, &sid, &eid)
            .await
            .map_err(internal_err)?;
        let body = json!({
            "service_id": sid.as_str(),
            "deploy_id": deploy_id.as_str(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(
        description = "Permanently delete a Railway service. Requires `confirm: true` (R9). Irreversible."
    )]
    async fn railway_service_delete(
        &self,
        Parameters(req): Parameters<DeleteRequest>,
    ) -> Result<CallToolResult, McpError> {
        if !req.confirm {
            return Err(McpError::invalid_params(
                "refusing to delete service without `confirm: true` (R9)".to_string(),
                None,
            ));
        }
        let client = build_client_for(req.account.as_deref())?;
        let sid = ServiceId::from(req.service);
        M::service_delete(&client, &sid)
            .await
            .map_err(internal_err)?;
        let body = json!({
            "deleted_service_id": sid.as_str(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(
        description = "Append a single line to the local L7 experience log (.trinity/experience/<YYYYMMDD>.trinity)."
    )]
    async fn railway_experience_append(
        &self,
        Parameters(req): Parameters<ExperienceAppendRequest>,
    ) -> Result<CallToolResult, McpError> {
        let project = req.project.unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let pid = ProjectId::from(project);
        let service_id = req.service.map(ServiceId::from);
        let token_fp = std::env::var("RAILWAY_TOKEN").ok().as_deref().map_or_else(
            || "no-token".to_string(),
            trios_railway_core::hash::token_fingerprint,
        );

        let verb = req.verb.unwrap_or_else(|| "experience".to_string());
        let hash = RailwayHash::seal(&verb, &pid, service_id.as_ref(), &token_fp);
        let agent = req.agent.unwrap_or_else(|| "GENERAL".to_string());
        let soul = req.soul_name.unwrap_or_else(|| "RailRangerOne".to_string());
        let status = req.status.unwrap_or_else(|| "OK".to_string());
        let line = ExperienceLine::from_hash(
            &agent,
            &soul,
            &req.issue,
            &req.task,
            &status,
            &req.phi_step,
            &hash,
        )
        .map_err(internal_err)?;
        let root: PathBuf = req.root.map_or_else(|| PathBuf::from("."), PathBuf::from);
        let path = append_line(&root.join(".trinity"), &line)
            .await
            .map_err(internal_err)?;

        let body = json!({
            "experience_path": path.display().to_string(),
            "triplet": hash.triplet(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(
        description = "Print the idempotent Neon DDL needed for the railway audit tables (issue #6)."
    )]
    async fn railway_audit_migrate_sql(&self) -> Result<CallToolResult, McpError> {
        let stmts = trios_railway_audit::migrations::ddl_statements();
        let sql = stmts
            .iter()
            .map(|s| format!("{s};"))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(CallToolResult::success(vec![Content::text(sql)]))
    }

    // =================================================================
    // mcp.<domain>.<verb> aliases — curated, tripwire-checked surface.
    //
    // Aliases share request types with the legacy `railway_*` tools so
    // operators can migrate gradually. New invariants enforced here:
    //   #107 project whitelist · #109 via_mcp marker · #110 audit
    //   append-only · #111 tool signature · #112 account-scoped writes
    //   · #113 dry-run default · #114 idempotency keys
    // =================================================================

    #[tool(
        name = "mcp.railway.list",
        description = "Curated alias of railway_service_list. Project whitelist (#107) is enforced."
    )]
    async fn mcp_railway_list(
        &self,
        Parameters(req): Parameters<ListServicesRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.railway.list", req.account.as_deref());
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.railway.list")?;
        let project = req.project.clone().unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        tripwires::t107_project_whitelist(&project)?;
        self.railway_service_list(Parameters(req)).await
    }

    #[tool(
        name = "mcp.railway.deploy",
        description = "Curated alias of railway_service_deploy. Requires explicit account (#112), project whitelist (#107), and idempotency_key (#114)."
    )]
    async fn mcp_railway_deploy(
        &self,
        Parameters(req): Parameters<McpDeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.railway.deploy", req.inner.account.as_deref());
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.railway.deploy")?;
        tripwires::t112_account_scoped(req.inner.account.as_deref())?;
        let project = req.inner.project.clone().unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        tripwires::t107_project_whitelist(&project)?;
        match tripwires::t114_idempotency_key("mcp.railway.deploy", Some(&req.idempotency_key))? {
            tripwires::IdempotencyOutcome::Replay => {
                let body = json!({
                    "replay": true,
                    "idempotency_key": req.idempotency_key,
                    "note": "already executed in this MCP process; Railway is source-of-truth for actual state",
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&body).unwrap(),
                )]));
            }
            tripwires::IdempotencyOutcome::First => {}
        }
        self.railway_service_deploy(Parameters(req.inner)).await
    }

    #[tool(
        name = "mcp.railway.redeploy",
        description = "Curated alias of railway_service_redeploy. Requires explicit account (#112) and idempotency_key (#114)."
    )]
    async fn mcp_railway_redeploy(
        &self,
        Parameters(req): Parameters<McpRedeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.railway.redeploy", req.inner.account.as_deref());
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.railway.redeploy")?;
        tripwires::t112_account_scoped(req.inner.account.as_deref())?;
        match tripwires::t114_idempotency_key(
            "mcp.railway.redeploy",
            Some(&req.idempotency_key),
        )? {
            tripwires::IdempotencyOutcome::Replay => {
                let body = json!({
                    "replay": true,
                    "idempotency_key": req.idempotency_key,
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&body).unwrap(),
                )]));
            }
            tripwires::IdempotencyOutcome::First => {}
        }
        self.railway_service_redeploy(Parameters(req.inner)).await
    }

    #[tool(
        name = "mcp.railway.delete",
        description = "Curated alias of railway_service_delete. Requires explicit account (#112), dry-run default (#113), and confirm=true to actually delete."
    )]
    async fn mcp_railway_delete(
        &self,
        Parameters(req): Parameters<McpDeleteRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.railway.delete", req.inner.account.as_deref());
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.railway.delete")?;
        tripwires::t112_account_scoped(req.inner.account.as_deref())?;
        tripwires::t113_dry_run_default(req.inner.confirm, req.dry_run)?;
        if req.dry_run.unwrap_or(true) {
            let body = json!({
                "dry_run": true,
                "would_delete": req.inner.service,
                "account": req.inner.account,
                "note": "set dry_run=false AND confirm=true to actually delete",
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&body).unwrap(),
            )]));
        }
        self.railway_service_delete(Parameters(req.inner)).await
    }

    #[tool(
        name = "mcp.experience.append",
        description = "Curated alias of railway_experience_append. Audit ledger is append-only (#110); destructive verbs in `task` are rejected."
    )]
    async fn mcp_experience_append(
        &self,
        Parameters(req): Parameters<ExperienceAppendRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.experience.append", None);
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.experience.append")?;
        tripwires::t110_audit_append_only(&req.task)?;
        self.railway_experience_append(Parameters(req)).await
    }

    #[tool(
        name = "mcp.audit.migrate",
        description = "Curated alias of railway_audit_migrate_sql. Read-only; emits the idempotent DDL string."
    )]
    async fn mcp_audit_migrate(&self) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.audit.migrate", None);
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.audit.migrate")?;
        self.railway_audit_migrate_sql().await
    }

    // =================================================================
    // mcp.igla.validate — pure parser/validator for IGLA canonical names
    // =================================================================

    #[tool(
        name = "mcp.igla.validate",
        description = "Parse an IGLA canonical service name (`IGLA-<MODEL>-<FORMAT>-E<NNNN>[-<TAG>]-rng<SEED>`) and validate it against L-R8 / L-R9 / L-METRIC plus tripwires #98/#99/#100. Read-only."
    )]
    async fn mcp_igla_validate(
        &self,
        Parameters(req): Parameters<IglaValidateRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.igla.validate", None);
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.igla.validate")?;
        let canon = req
            .name
            .parse::<IglaCanon>()
            .map_err(|e| McpError::invalid_params(format!("canon parse: {e}"), None))?;
        let mut errors: Vec<String> = Vec::new();
        if let Some(h) = req.capacity_h {
            if let Err(e) = canon.validate_with_capacity(h) {
                errors.push(format!("capacity: {e}"));
            }
        }
        if let Some(loss) = req.primary_loss_kind.as_deref() {
            if let Err(e) = canon.enforce_l_metric(loss) {
                errors.push(format!("l_metric: {e}"));
            }
        }
        if let Some(current_max) = req.current_max_exp_id {
            if let Err(e) = canon.validate_for_deploy(current_max) {
                errors.push(format!("deploy: {e}"));
            }
        }
        let body = json!({
            "input": req.name,
            "parsed": canon.to_string(),
            "model": format!("{:?}", canon.model),
            "format": format!("{:?}", canon.format),
            "exp_id": canon.exp_id,
            "tag": canon.tag,
            "rng": canon.rng,
            "legacy_seed": canon.legacy_seed,
            "errors": errors,
            "valid": errors.is_empty(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    // =================================================================
    // mcp.fleet.snapshot — fan-out service inventory across accounts
    // =================================================================

    #[tool(
        name = "mcp.fleet.snapshot",
        description = "Read-only snapshot of services across one or all registered accounts. Pass `account=acc{0,1,2,3}` for one account, omit for all."
    )]
    async fn mcp_fleet_snapshot(
        &self,
        Parameters(req): Parameters<FleetSnapshotRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.fleet.snapshot", req.account.as_deref());
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.fleet.snapshot")?;

        let mc = RailwayMultiClient::from_env().map_err(|e| {
            McpError::internal_error(
                format!("failed to load RailwayMultiClient: {e}"),
                None,
            )
        })?;

        let targets: Vec<AccountId> = match req.account.as_deref() {
            Some(a) => vec![AccountId::from_alias(a).ok_or_else(|| {
                McpError::invalid_params(
                    format!("unknown account alias {a:?}; expected acc0/acc1/acc2/acc3"),
                    None,
                )
            })?],
            None => mc.registered(),
        };

        let mut accounts_out: Vec<serde_json::Value> = Vec::new();
        let mut total_services: usize = 0;
        for id in targets {
            let creds = match mc.creds(id) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let project = creds.project.as_str().to_string();
            // Project whitelist (#107) — skip any non-allowed project.
            if assert_project_allowed(&project).is_err() {
                accounts_out.push(json!({
                    "account": id.as_str(),
                    "project": project,
                    "skipped": "not in ALLOWED_PROJECT_IDS",
                }));
                continue;
            }
            let client = match mc.get(id) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let pid = ProjectId::from(project.clone());
            match Q::project_view(client, &pid).await {
                Ok(pv) => {
                    let services: Vec<_> = pv
                        .services()
                        .into_iter()
                        .map(|s| {
                            json!({
                                "id": s.id,
                                "name": s.name,
                                "created_at": s.created_at,
                            })
                        })
                        .collect();
                    total_services += services.len();
                    accounts_out.push(json!({
                        "account": id.as_str(),
                        "project_id": pv.id,
                        "project_name": pv.name,
                        "services": services,
                        "count": services.len(),
                    }));
                }
                Err(e) => {
                    accounts_out.push(json!({
                        "account": id.as_str(),
                        "project": project,
                        "error": format!("{e}"),
                    }));
                }
            }
        }

        let body = json!({
            "accounts": accounts_out,
            "total_services": total_services,
            "connection_summary": connections::render_summary_line(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    // =================================================================
    // mcp.fleet.cleanup — pattern-gated service cleanup
    // =================================================================

    #[tool(
        name = "mcp.fleet.cleanup",
        description = "Cull services in an account whose name does NOT match `keep_pattern`. Required: account (#112), keep_pattern (no nuclear delete-all), dry_run default true (#113), confirm=true to mutate, idempotency_key (#114)."
    )]
    async fn mcp_fleet_cleanup(
        &self,
        Parameters(req): Parameters<FleetCleanupRequest>,
    ) -> Result<CallToolResult, McpError> {
        connections::log_call("mcp", "mcp.fleet.cleanup", Some(&req.account));
        tripwires::t109_no_direct_call(VIA_MCP)?;
        tripwires::t111_tool_signature("mcp.fleet.cleanup")?;
        tripwires::t112_account_scoped(Some(&req.account))?;
        tripwires::t113_dry_run_default(req.confirm, req.dry_run)?;
        if req.keep_pattern.trim().is_empty() {
            return Err(McpError::invalid_params(
                "refusing nuclear cleanup: keep_pattern must be non-empty".to_string(),
                None,
            ));
        }
        let regex = regex::Regex::new(&req.keep_pattern).map_err(|e| {
            McpError::invalid_params(format!("keep_pattern not a valid regex: {e}"), None)
        })?;
        match tripwires::t114_idempotency_key("mcp.fleet.cleanup", Some(&req.idempotency_key))? {
            tripwires::IdempotencyOutcome::Replay => {
                let body = json!({
                    "replay": true,
                    "idempotency_key": req.idempotency_key,
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&body).unwrap(),
                )]));
            }
            tripwires::IdempotencyOutcome::First => {}
        }

        let id = AccountId::from_alias(&req.account).ok_or_else(|| {
            McpError::invalid_params(
                format!("unknown account alias {:?}", req.account),
                None,
            )
        })?;
        let mc = RailwayMultiClient::from_env().map_err(|e| {
            McpError::internal_error(
                format!("failed to load RailwayMultiClient: {e}"),
                None,
            )
        })?;
        let creds = mc.creds(id).map_err(|e| {
            McpError::internal_error(format!("account {id:?} not registered: {e}"), None)
        })?;
        let project = creds.project.as_str().to_string();
        tripwires::t107_project_whitelist(&project)?;
        let client = mc.get(id).map_err(|e| {
            McpError::internal_error(format!("account {id:?} not registered: {e}"), None)
        })?;
        let pid = ProjectId::from(project);
        let pv = Q::project_view(client, &pid).await.map_err(internal_err)?;

        let mut would_delete: Vec<serde_json::Value> = Vec::new();
        let mut keep: Vec<serde_json::Value> = Vec::new();
        for s in pv.services() {
            if regex.is_match(&s.name) {
                keep.push(json!({"id": s.id, "name": s.name}));
            } else {
                would_delete.push(json!({"id": s.id, "name": s.name}));
            }
        }

        let dry = req.dry_run.unwrap_or(true);
        if dry {
            let body = json!({
                "dry_run": true,
                "account": req.account,
                "keep_pattern": req.keep_pattern,
                "would_delete": would_delete,
                "keep": keep,
                "would_delete_count": would_delete.len(),
                "keep_count": keep.len(),
                "note": "set dry_run=false AND confirm=true to actually delete",
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&body).unwrap(),
            )]));
        }

        let mut deleted: Vec<String> = Vec::new();
        let mut errors: Vec<serde_json::Value> = Vec::new();
        for v in &would_delete {
            let sid_str = v["id"].as_str().unwrap_or("").to_string();
            if sid_str.is_empty() {
                continue;
            }
            let sid = ServiceId::from(sid_str.clone());
            match M::service_delete(client, &sid).await {
                Ok(_) => deleted.push(sid_str),
                Err(e) => errors.push(json!({"service": sid_str, "error": format!("{e}")})),
            }
        }

        let body = json!({
            "dry_run": false,
            "account": req.account,
            "keep_pattern": req.keep_pattern,
            "deleted": deleted,
            "errors": errors,
            "deleted_count": deleted.len(),
            "kept_count": keep.len(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }
}

impl Default for TriosRailwayMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for TriosRailwayMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "trios-railway-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Trios Railway MCP".to_string()),
                website_url: Some("https://github.com/gHashTag/trios-railway".to_string()),
                icons: None,
            },
            instructions: Some(
                "Public MCP server controlling the IGLA Railway project. \
                 Set RAILWAY_TOKEN before invoking deploy/redeploy/delete tools. \
                 Anchor: phi^2 + phi^-2 = 3."
                    .to_string(),
            ),
        }
    }
}

// -------- helpers --------

fn build_client() -> Result<Client, McpError> {
    Client::from_env().map_err(|e| {
        McpError::internal_error(format!("RAILWAY_TOKEN not set or invalid: {e}"), None)
    })
}

/// Resolve a `Client` for the requested account alias. When `alias` is
/// `None` we keep the legacy single-token path so existing one-account
/// deployments keep working untouched.
fn build_client_for(alias: Option<&str>) -> Result<Client, McpError> {
    let Some(alias) = alias else {
        return build_client();
    };
    let id = AccountId::from_alias(alias).ok_or_else(|| {
        McpError::invalid_params(
            format!(
                "unknown account alias {alias:?}; expected acc0/acc1/acc2/acc3"
            ),
            None,
        )
    })?;
    let mc = RailwayMultiClient::from_env().map_err(|e| {
        McpError::internal_error(
            format!("failed to load RailwayMultiClient from env: {e}"),
            None,
        )
    })?;
    let client = mc.get(id).map_err(|e| {
        McpError::internal_error(
            format!(
                "account {alias:?} not authorized in this MCP instance: {e}. Set RAILWAY_TOKEN_{} (and _PROJECT_ID_/_ENVIRONMENT_ID_/_TOKEN_KIND_).",
                alias.to_ascii_uppercase()
            ),
            None,
        )
    })?;
    Ok(client.clone())
}

fn internal_err<E: std::fmt::Display>(e: E) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

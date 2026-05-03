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
    mutations as M, queries as Q, transport::Client, EnvironmentId, ProjectId, RailwayHash,
    ServiceId,
};
use trios_railway_experience::{append_line, ExperienceLine};

const IGLA_PROJECT_ID: &str = "f29aa9dd-ca0b-460f-ad24-c7680c6717fb";
const IGLA_PROD_ENV_ID: &str = "fade0d77-af80-4d01-bc34-2ce27283d766";
const DEFAULT_TRAINER_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

// -------- request payload structs --------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListServicesRequest {
    /// Project UUID. Defaults to the IGLA project.
    #[serde(default)]
    pub project: Option<String>,
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
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
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
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteRequest {
    /// Service UUID to delete.
    pub service: String,
    /// Must be `true` (R9 safety): the call refuses to proceed otherwise.
    pub confirm: bool,
}

/// Built-in template that expands into N Railway services on a single MCP call.
///
/// Currently supported:
/// - `champion-repro`: champion config (h=384, lr=0.003, adamw, `27_000` steps, `attn_layers=2`,
///   A-champion-fineweb lane). One service per seed.
/// - `gate2-final`: gate-2 final config (`30_000` steps, jepa+nca aux objectives).
///   One service per seed.
/// - `e2e-ttt-track-10min`: E2E TTT WIN sweep (`track_10min_16mb`, early-stop at 1.07063).
///   One service per seed; image is the trainer-igla bundle. Useful for RTX-class hosts (Railway
///   GPU or `RunPod` via the same image).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TemplateDeployRequest {
    /// Template id, see `TemplateDeployRequest` doc-comment.
    pub template: String,
    /// One Railway service is created per seed. Service name = `<template>-rng<seed>`.
    pub seeds: Vec<i64>,
    /// Optional `canon_name` prefix override.
    /// Default `canon_name` pattern: `IGLA-<TEMPLATE_UPPER>-rng<seed>`.
    #[serde(default)]
    pub canon_prefix: Option<String>,
    /// Optional Docker image. Defaults to the canonical IGLA trainer image.
    #[serde(default)]
    pub image: Option<String>,
    /// Optional project UUID. Defaults to the MCP project.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional environment UUID. Defaults to the MCP env.
    #[serde(default)]
    pub environment: Option<String>,
    /// Extra/override env-var pairs applied AFTER template defaults.
    /// Only `NEON_DATABASE_URL` is required — all DSN aliases are derived
    /// from it automatically (no more `TRIOS_NEON_DSN` / `DATABASE_URL` duplication).
    #[serde(default)]
    pub vars_override: Vec<KeyValue>,
    /// Wave name written into the WAVE env var. Auto-generated when omitted.
    #[serde(default)]
    pub wave: Option<String>,
    /// `DOC_ID` for the audit ledger. Auto-generated when omitted.
    #[serde(default)]
    pub doc_id: Option<String>,
    /// Repo root for the L7 experience log. Defaults to `.`.
    #[serde(default)]
    pub experience_root: Option<String>,
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
        let client = build_client()?;
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
        let client = build_client()?;
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
        let client = build_client()?;
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
        let client = build_client()?;
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
        description = "Deploy a built-in IGLA training template across N seeds in one call. \
                       Supported templates: 'champion-repro' (27K steps), 'gate2-final' (30K steps + jepa/nca), \
                       'e2e-ttt-track-10min' (WIN sweep with early-stop at 1.07063). \
                       Creates one Railway service per seed, applies template defaults plus vars_override, \
                       triggers redeploys, and emits one R7 audit triplet per service. Idempotent on service name. \
                       Only NEON_DATABASE_URL is required in vars_override — no DSN aliases needed."
    )]
    async fn railway_template_deploy(
        &self,
        Parameters(req): Parameters<TemplateDeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        if req.seeds.is_empty() {
            return Err(McpError::invalid_params(
                "seeds[] must contain at least one seed".to_string(),
                None,
            ));
        }

        let template = req.template.trim().to_lowercase();
        let template_defaults: Vec<KeyValue> = match template.as_str() {
            "champion-repro" => vec![
                kv("TRIOS_HIDDEN", "384"),
                kv("TRIOS_LR", "0.003"),
                kv("TRIOS_OPTIMIZER", "adamw"),
                kv("TRIOS_STEPS", "27000"),
                kv("TRIOS_ATTN_LAYERS", "2"),
                kv("TRIOS_CHECKPOINT_INTERVAL", "100"),
                kv("TRIOS_LANE", "A-champion-fineweb"),
                kv("L_R8_SYNTHETIC_FALLBACK", "FORBID"),
                kv("RAILWAY_ACC", "acc0"),
                kv("RUST_LOG", "info"),
            ],
            "gate2-final" => vec![
                kv("TRIOS_HIDDEN", "384"),
                kv("TRIOS_LR", "0.004"),
                kv("TRIOS_OPTIMIZER", "adamw"),
                kv("TRIOS_STEPS", "30000"),
                kv("TRIOS_ATTN_LAYERS", "3"),
                kv("TRIOS_CHECKPOINT_INTERVAL", "100"),
                kv("TRIOS_W_CE", "1.0"),
                kv("TRIOS_W_JEPA", "0.15"),
                kv("TRIOS_W_NCA", "0.10"),
                kv("TRIOS_LANE", "B-gate2-final"),
                kv("L_R8_SYNTHETIC_FALLBACK", "FORBID"),
                kv("RAILWAY_ACC", "acc0"),
                kv("RUST_LOG", "info"),
            ],
            "e2e-ttt-track-10min" => vec![
                kv("OBJECTIVE", "E2E_TTT"),
                kv("TRACK", "track_10min_16mb"),
                kv("TRIOS_CHECKPOINT_INTERVAL", "100"),
                kv("EARLY_STOP_BPB", "1.07063"),
                kv("TRIOS_LANE", "C-e2e-ttt-win"),
                kv("RAILWAY_ACC", "acc0"),
                kv("RUST_LOG", "info"),
            ],
            other => {
                return Err(McpError::invalid_params(
                    format!(
                        "unknown template '{other}'. supported: champion-repro, gate2-final, e2e-ttt-track-10min"
                    ),
                    None,
                ));
            }
        };

        let canon_prefix = req
            .canon_prefix
            .clone()
            .unwrap_or_else(|| format!("IGLA-{}", template.to_uppercase()));
        let wave = req
            .wave
            .clone()
            .unwrap_or_else(|| format!("EPIC-446-{}", template.to_uppercase()));
        let doc_id = req
            .doc_id
            .clone()
            .unwrap_or_else(|| format!("EPIC-446-TEMPLATE-{}", template.to_uppercase()));

        let project = req
            .project
            .clone()
            .unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let environment = req
            .environment
            .clone()
            .unwrap_or_else(|| IGLA_PROD_ENV_ID.to_string());
        let image = req
            .image
            .clone()
            .unwrap_or_else(|| DEFAULT_TRAINER_IMAGE.to_string());

        let client = build_client()?;
        let token_fp = client.token_fingerprint();
        let pid = ProjectId::from(project.clone());
        let eid = EnvironmentId::from(environment.clone());

        let exp_root: PathBuf = req
            .experience_root
            .clone()
            .map_or_else(|| PathBuf::from("."), PathBuf::from);

        let mut deployed: Vec<serde_json::Value> = Vec::with_capacity(req.seeds.len());
        let mut errors: Vec<serde_json::Value> = Vec::new();

        for seed in &req.seeds {
            let svc_name = format!("{template}-rng{seed}");
            let canon_name = format!("{canon_prefix}-rng{seed}");

            // 1. find-or-create service
            let service_id: ServiceId = match find_service_by_name(&client, &pid, &svc_name).await {
                Ok(Some(existing)) => existing,
                Ok(None) => match M::service_create(&client, &pid, &svc_name).await {
                    Ok(c) => ServiceId::from(c.id),
                    Err(e) => {
                        errors.push(json!({"seed": seed, "stage": "create", "err": e.to_string()}));
                        continue;
                    }
                },
                Err(e) => {
                    errors.push(json!({"seed": seed, "stage": "lookup", "err": e.to_string()}));
                    continue;
                }
            };

            // 2. pin image
            if let Err(e) = M::service_instance_set_image(&client, &service_id, &eid, &image).await
            {
                errors.push(json!({"seed": seed, "stage": "set_image", "err": e.to_string()}));
                continue;
            }

            // 3. assemble env: template defaults + per-seed overrides + caller overrides
            //    DSN policy: NEON_DATABASE_URL is the single source of truth.
            //    TRIOS_CANON_NAME (not CANON_NAME) is what train_loop.rs reads.
            let mut env: Vec<KeyValue> = template_defaults.clone();
            env.push(kv("TRIOS_SEED", &seed.to_string()));
            env.push(kv("TRIOS_CANON_NAME", &canon_name)); // fix: was CANON_NAME
            env.push(kv("WAVE", &wave));
            env.push(kv("DOC_ID", &doc_id));
            for kv_pair in &req.vars_override {
                // override semantics: replace if key exists
                if let Some(slot) = env.iter_mut().find(|e| e.key == kv_pair.key) {
                    slot.value.clone_from(&kv_pair.value);
                } else {
                    env.push(KeyValue {
                        key: kv_pair.key.clone(),
                        value: kv_pair.value.clone(),
                    });
                }
            }

            // 4. upsert env
            let mut upsert_err = None;
            for var in &env {
                if let Err(e) =
                    M::variable_upsert(&client, &pid, &eid, &service_id, &var.key, &var.value).await
                {
                    upsert_err = Some((var.key.clone(), e.to_string()));
                    break;
                }
            }
            if let Some((key, err)) = upsert_err {
                errors.push(json!({
                    "seed": seed,
                    "stage": "variable_upsert",
                    "key": key,
                    "err": err,
                }));
                continue;
            }

            // 5. redeploy
            let deploy_id = match M::service_redeploy(&client, &service_id, &eid).await {
                Ok(d) => d,
                Err(e) => {
                    errors.push(json!({"seed": seed, "stage": "redeploy", "err": e.to_string()}));
                    continue;
                }
            };

            // 6. R7 audit triplet
            let hash = RailwayHash::seal("template-deploy", &pid, Some(&service_id), &token_fp);
            if let Ok(line) = ExperienceLine::from_hash(
                "GENERAL",
                "RailRangerOne",
                "#20",
                &format!("mcp template-deploy {template} seed={seed} canon={canon_name}"),
                "OK",
                "PUSH",
                &hash,
            ) {
                let _ = append_line(&exp_root.join(".trinity"), &line).await;
            }

            deployed.push(json!({
                "seed": seed,
                "service_id": service_id.as_str(),
                "service_name": svc_name,
                "canon_name": canon_name,
                "deploy_id": deploy_id.as_str(),
                "triplet": hash.triplet(),
            }));
        }

        let body = json!({
            "template": template,
            "image": image,
            "project_id": project,
            "environment_id": environment,
            "wave": wave,
            "doc_id": doc_id,
            "deployed": deployed,
            "deployed_count": deployed.len(),
            "errors": errors,
            "error_count": errors.len(),
            "anchor": "phi^2 + phi^-2 = 3",
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
                 Only NEON_DATABASE_URL is needed — no DSN aliases required. \
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

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: value.to_string(),
    }
}

async fn find_service_by_name(
    client: &Client,
    pid: &ProjectId,
    name: &str,
) -> Result<Option<ServiceId>, anyhow::Error> {
    let pv = Q::project_view(client, pid).await?;
    Ok(pv
        .services()
        .into_iter()
        .find(|s| s.name == name)
        .map(|s| ServiceId::from(s.id)))
}

fn internal_err<E: std::fmt::Display>(e: E) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

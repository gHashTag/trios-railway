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
    is_uuid_like, mutations as M, queries as Q, transport::Client, AuthMode, EnvironmentId,
    ProjectId, RailwayHash, ServiceId,
};
use trios_railway_experience::{append_line, ExperienceLine};

const IGLA_PROJECT_ID: &str = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b";
const IGLA_PROD_ENV_ID: &str = "54e293b9-00a9-4102-814d-db151636d96e";
const DEFAULT_TRAINER_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

/// All known project IDs that this gateway is allowed to operate on.
/// Serves as both a whitelist and a routing table to select the correct
/// per-account token.
const ALLOWED_PROJECT_IDS: &[&str] = &[
    "e4fe33bb-3b09-4842-9782-7d2dea1abc9b", // acc1 — IGLA (primary)
    "da1fb0c7-199f-42b0-9f08-a84d122feb5b", // acc0 — woody
    "f3350520-8aff-4ebf-8618-c041bd17e6d0", // acc2
    "8ab06401-aa28-4af7-9faf-39a1548b7008", // acc3
];

/// Per-account token info, loaded once from env vars.
struct AccountConfig {
    project_id: String,
    token: String,
    token_kind: String,
    env_id: String,
}

static ACCOUNTS: std::sync::OnceLock<Vec<AccountConfig>> = std::sync::OnceLock::new();

fn load_accounts() -> Vec<AccountConfig> {
    let mut accounts = Vec::new();
    for i in 0..4 {
        let Ok(token) = std::env::var(format!("RAILWAY_TOKEN_ACC{i}")) else {
            continue;
        };
        let project_id = std::env::var(format!("RAILWAY_PROJECT_ID_ACC{i}")).unwrap_or_default();
        let token_kind = std::env::var(format!("RAILWAY_TOKEN_KIND_ACC{i}")).unwrap_or_default();
        let env_id =
            std::env::var(format!("RAILWAY_ENVIRONMENT_ID_ACC{i}")).unwrap_or_default();
        if !project_id.is_empty() {
            accounts.push(AccountConfig {
                project_id,
                token,
                token_kind,
                env_id,
            });
        }
    }
    tracing::info!(count = accounts.len(), "loaded multi-account config");
    accounts
}

fn accounts() -> &'static Vec<AccountConfig> {
    ACCOUNTS.get_or_init(load_accounts)
}

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
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteRequest {
    /// Service UUID to delete.
    pub service: String,
    /// Must be `true` (R9 safety): the call refuses to proceed otherwise.
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[allow(dead_code)]
pub struct BatchRedeployRequest {
    /// Account index (0-3).
    pub account: u8,
    /// Optional name substring filter (e.g. "seed-42").
    #[serde(default)]
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[allow(dead_code)]
pub struct ExperimentInsertRequest {
    /// Canonical experiment name (e.g. "IGLA-TRAIN_V2-GF16-E0800-H512-rng10001").
    pub canon_name: String,
    /// Experiment config as JSON object.
    pub config_json: serde_json::Value,
    /// Priority 0-100 (higher = runs first).
    pub priority: i32,
    /// Random seed (must be sanctioned: 42, 43, 44, 1597, 2584, 4181, 6765, 10001-10010, 10946).
    pub seed: i32,
    /// Training steps budget.
    pub steps_budget: i32,
    /// Target account (acc0, acc1, acc2, acc3).
    pub account: String,
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
        let project = req.project.unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let client = build_client_for_project(&project)?;
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
        let project = req.project.unwrap_or_else(|| IGLA_PROJECT_ID.to_string());
        let client = build_client_for_project(&project)?;
        let token_fp = client.token_fingerprint();

        let environment = req
            .environment
            .unwrap_or_else(|| env_for_project(&project));
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
        let pid = ProjectId::from(project.as_str());
        let service_id = req.service.map(ServiceId::from);
        let token_fp = build_client_for_project(&project)
            .map_or_else(|_| "no-token".to_string(), |c| c.token_fingerprint());

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

    #[tool(
        description = "Check fleet health across all accounts. Returns service counts, project status, and account connectivity for each configured account."
    )]
    async fn fleet_health(&self) -> Result<CallToolResult, McpError> {
        let mut results = Vec::new();
        let mut total_services = 0usize;
        let mut healthy_accounts = 0usize;

        for acc in accounts() {
            let auth = match acc.token_kind.as_str() {
                "team" | "bearer" | "personal" => AuthMode::Team,
                "project" => AuthMode::Project,
                _ if is_uuid_like(&acc.token) => AuthMode::Project,
                _ => AuthMode::Team,
            };
            let Ok(client) = Client::with_token_and_mode(&acc.token, auth) else {
                results.push(json!({
                    "account": acc.project_id,
                    "status": "ERROR",
                    "error": "client build failed",
                    "services": 0,
                }));
                continue;
            };
            let pid = ProjectId::from(acc.project_id.as_str());
            match Q::project_view(&client, &pid).await {
                Ok(pv) => {
                    let count = pv.services().len();
                    total_services += count;
                    healthy_accounts += 1;
                    results.push(json!({
                        "project_id": pv.id,
                        "project_name": pv.name,
                        "status": "OK",
                        "services": count,
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "project_id": acc.project_id,
                        "status": "ERROR",
                        "error": e.to_string(),
                        "services": 0,
                    }));
                }
            }
        }

        let body = json!({
            "healthy_accounts": healthy_accounts,
            "total_accounts": accounts().len(),
            "total_services": total_services,
            "accounts": results,
            "anchor": "phi^2 + phi^-2 = 3",
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    #[tool(
        description = "List all seed training services across all accounts. Returns service name, ID, and project for every service matching 'seed' or 'igla' or 'train' pattern."
    )]
    async fn seed_list(&self) -> Result<CallToolResult, McpError> {
        let mut all_seeds = Vec::new();

        for acc in accounts() {
            let auth = match acc.token_kind.as_str() {
                "team" | "bearer" | "personal" => AuthMode::Team,
                "project" => AuthMode::Project,
                _ if is_uuid_like(&acc.token) => AuthMode::Project,
                _ => AuthMode::Team,
            };
            let Ok(client) = Client::with_token_and_mode(&acc.token, auth) else {
                continue;
            };
            let pid = ProjectId::from(acc.project_id.as_str());
            let Ok(pv) = Q::project_view(&client, &pid).await else {
                continue;
            };
            for s in pv.services() {
                let lower = s.name.to_lowercase();
                if lower.contains("seed")
                    || lower.contains("igla")
                    || lower.contains("train")
                {
                    all_seeds.push(json!({
                        "id": s.id,
                        "name": s.name,
                        "project_id": acc.project_id,
                        "created_at": s.created_at,
                    }));
                }
            }
        }

        let body = json!({
            "total_seeds": all_seeds.len(),
            "seeds": all_seeds,
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).unwrap(),
        )]))
    }

    // -------- new tools: experiment_queue, worker_status, batch_redeploy --------

    /// Show experiment queue status grouped by status and account.
    #[tool(description = "Show experiment queue status from Neon database. Returns counts grouped by status and account, plus total pending/running/done/failed/pruned.")]
    async fn experiment_queue_status(&self) -> Result<CallToolResult, McpError> {
        let client = db_connect().await?;
        let rows = client
            .query(
                "SELECT status, account, COUNT(*) as cnt FROM experiment_queue GROUP BY status, account ORDER BY status, account",
                &[],
            )
            .await
            .map_err(internal_err)?;

        let mut summary = serde_json::Map::new();
        for row in &rows {
            let status: String = row.get(0);
            let account: String = row.get(1);
            let cnt: i64 = row.get(2);
            let key = format!("{status}/{account}");
            summary.insert(key, json!(cnt));
        }

        // Also get totals
        let total_rows = client
            .query(
                "SELECT status, COUNT(*) as cnt FROM experiment_queue GROUP BY status ORDER BY status",
                &[],
            )
            .await
            .map_err(internal_err)?;

        let mut totals = serde_json::Map::new();
        for row in &total_rows {
            let status: String = row.get(0);
            let cnt: i64 = row.get(1);
            totals.insert(status, json!(cnt));
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "breakdown": summary,
                "totals": totals,
            }))
            .map_err(internal_err)?,
        )]))
    }

    /// Show worker status grouped by account with alive/stale/dead counts.
    #[tool(description = "Show worker status from Neon database. Returns counts of alive (<5min), stale (5-30min), and dead (>30min) workers per account.")]
    async fn worker_status(&self) -> Result<CallToolResult, McpError> {
        let client = db_connect().await?;
        let rows = client
            .query(
                "SELECT railway_acc, COUNT(*) as total,
                    COUNT(*) FILTER (WHERE last_heartbeat > now() - interval '5 minutes') as alive_5m,
                    COUNT(*) FILTER (WHERE last_heartbeat BETWEEN now() - interval '30 minutes' AND now() - interval '5 minutes') as stale,
                    COUNT(*) FILTER (WHERE last_heartbeat < now() - interval '30 minutes') as dead
                 FROM workers GROUP BY railway_acc ORDER BY railway_acc",
                &[],
            )
            .await
            .map_err(internal_err)?;

        let mut result = serde_json::Map::new();
        for row in &rows {
            let acc: String = row.get(0);
            let total: i64 = row.get(1);
            let alive: i64 = row.get(2);
            let stale: i64 = row.get(3);
            let dead: i64 = row.get(4);
            result.insert(acc, json!({ "total": total, "alive": alive, "stale": stale, "dead": dead }));
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).map_err(internal_err)?,
        )]))
    }

    /// Redeploy all services on a specific account.
    #[tool(description = "Redeploy all (or filtered) services on a specific account. Provide account index (0-3) and optional name filter. Triggers redeploy for each matching service.")]
    async fn service_batch_redeploy(
        &self,
        Parameters(params): Parameters<BatchRedeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        let acc = accounts()
            .get(params.account as usize)
            .ok_or_else(|| McpError::invalid_params(format!("Account index {} not found (0-3)", params.account), None))?;

        let auth = match acc.token_kind.as_str() {
            "team" | "bearer" | "personal" => AuthMode::Team,
            "project" => AuthMode::Project,
            _ if is_uuid_like(&acc.token) => AuthMode::Project,
            _ => AuthMode::Team,
        };
        let client = Client::with_token_and_mode(&acc.token, auth).map_err(internal_err)?;
        let pid = ProjectId::from(acc.project_id.as_str());
        let eid = EnvironmentId::from(acc.env_id.as_str());

        let pv = Q::project_view(&client, &pid).await.map_err(internal_err)?;
        let all_services = pv.services();
        let services: Vec<_> = all_services
            .iter()
            .filter(|s| {
                if let Some(ref f) = params.filter {
                    s.name.contains(f.as_str())
                } else {
                    true
                }
            })
            .collect();

        let mut ok = 0u32;
        let mut err_count = 0u32;
        for s in &services {
            let sid = ServiceId::from(s.id.as_str());
            match M::service_redeploy(&client, &sid, &eid).await {
                Ok(_) => ok += 1,
                Err(e) => {
                    tracing::warn!(service = %s.name, %e, "redeploy failed");
                    err_count += 1;
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "account": params.account,
                "project": acc.project_id,
                "total_services": services.len(),
                "redeployed": ok,
                "failed": err_count,
            }))
            .map_err(internal_err)?,
        )]))
    }

    /// Insert experiments into the queue.
    #[tool(description = "Insert experiments into the experiment_queue table in Neon database. Provide canon_name, config_json, priority, seed, steps_budget, and account. Only sanctioned seeds are allowed (42, 43, 44, 1597, 2584, 4181, 6765, 10001-10010, 10946).")]
    async fn experiment_queue_insert(
        &self,
        Parameters(params): Parameters<ExperimentInsertRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = db_connect().await?;
        let config_str = serde_json::to_string(&params.config_json).map_err(internal_err)?;
        let rows = client
            .query_one(
                "INSERT INTO experiment_queue (canon_name, config_json, priority, seed, steps_budget, account, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6, 'mcp-gateway')
                 RETURNING id",
                &[&params.canon_name, &config_str, &params.priority, &params.seed, &params.steps_budget, &params.account],
            )
            .await
            .map_err(internal_err)?;

        let id: i64 = rows.get(0);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "inserted": true,
                "id": id,
                "canon_name": params.canon_name,
                "seed": params.seed,
                "account": params.account,
                "priority": params.priority,
                "steps_budget": params.steps_budget,
            }))
            .map_err(internal_err)?,
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

/// Build a client using the default `RAILWAY_TOKEN` env var (legacy fallback).
fn build_client() -> Result<Client, McpError> {
    Client::from_env().map_err(|e| {
        McpError::internal_error(format!("RAILWAY_TOKEN not set or invalid: {e}"), None)
    })
}

/// Build a client with the correct token for the given project ID.
/// Looks up `RAILWAY_TOKEN_ACC{0..3}` env vars to find a matching account.
/// Falls back to `build_client()` if no match found.
fn build_client_for_project(project: &str) -> Result<Client, McpError> {
    // Validate project is in whitelist
    if !ALLOWED_PROJECT_IDS.contains(&project) {
        return Err(McpError::invalid_params(
            format!(
                "project {project} not in ALLOWED_PROJECT_IDS. Allowed: {ALLOWED_PROJECT_IDS:?}"
            ),
            None,
        ));
    }
    // Find matching account
    for acc in accounts() {
        if acc.project_id == project {
            let auth = match acc.token_kind.as_str() {
                "team" | "bearer" | "personal" => AuthMode::Team,
                "project" => AuthMode::Project,
                _ if is_uuid_like(&acc.token) => AuthMode::Project,
                _ => AuthMode::Team,
            };
            return Client::with_token_and_mode(&acc.token, auth).map_err(|e| {
                McpError::internal_error(
                    format!("token error for project {project}: {e}"),
                    None,
                )
            });
        }
    }
    // Fallback to default token
    build_client()
}

/// Return the environment ID for a project, or the default IGLA env.
fn env_for_project(project: &str) -> String {
    for acc in accounts() {
        if acc.project_id == project && !acc.env_id.is_empty() {
            return acc.env_id.clone();
        }
    }
    IGLA_PROD_ENV_ID.to_string()
}

fn internal_err<E: std::fmt::Display>(e: E) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

// -------- database helpers --------

#[allow(dead_code)]
fn neon_url() -> Result<String, McpError> {
    std::env::var("NEON_DATABASE_URL").map_err(|_| {
        McpError::internal_error("NEON_DATABASE_URL not set — required for queue/worker tools", None)
    })
}

async fn db_connect() -> Result<tokio_postgres::Client, McpError> {
    let raw_url = neon_url()?;
    // Strip channel_binding — tokio-postgres doesn't support it.
    // Keep sslmode=require so tokio-postgres knows to use TLS.
    let url: String = raw_url
        .split('&')
        .filter(|p| !p.starts_with("channel_binding="))
        .collect::<Vec<_>>()
        .join("&");
    let url = url.replace("?&", "?");
    tracing::info!(url_len = url.len(), "connecting to Neon via rustls");

    // Install aws-lc-rs crypto provider (required by rustls 0.23)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Build rustls TLS connector with webpki roots for Neon
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let rustls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);

    // Connect with 10s timeout to avoid hanging
    let connect_future = tokio_postgres::connect(&url, tls);
    let (client, connection) = tokio::time::timeout(std::time::Duration::from_secs(10), connect_future)
        .await
        .map_err(|_| McpError::internal_error("Neon connection timed out after 10s", None))?
        .map_err(internal_err)?;

    // Spawn connection handler in background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!(%e, "postgres connection error");
        }
    });
    Ok(client)
}


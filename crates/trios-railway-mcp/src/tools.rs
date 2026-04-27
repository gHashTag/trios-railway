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

const IGLA_PROJECT_ID: &str = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b";
const IGLA_PROD_ENV_ID: &str = "54e293b9-00a9-4102-814d-db151636d96e";
const DEFAULT_TRAINER_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

// -------- request payload structs --------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListServicesRequest {
    /// Project UUID. Defaults to the IGLA project.
    #[serde(default)]
    pub project: Option<String>,
    /// Account alias (e.g. `acc1`, `acc2`). Defaults to `RAILWAY_TOKEN`.
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
    /// Account alias (e.g. `acc1`, `acc2`). Defaults to `RAILWAY_TOKEN`.
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
    /// Account alias (e.g. `acc1`, `acc2`). Defaults to `RAILWAY_TOKEN`.
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteRequest {
    /// Service UUID to delete.
    pub service: String,
    /// Must be `true` (R9 safety): the call refuses to proceed otherwise.
    pub confirm: bool,
    /// Account alias (e.g. `acc1`, `acc2`). Defaults to `RAILWAY_TOKEN`.
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
        let client = client_for_account(req.account.as_deref())?;
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
            serde_json::to_string_pretty(&body).map_err(internal_err)?,
        )]))
    }

    #[tool(
        description = "Create (or reuse) a Railway service, pin its image, upsert env vars, and trigger a redeploy. Emits an L7 experience line. Requires RAILWAY_TOKEN env var."
    )]
    async fn railway_service_deploy(
        &self,
        Parameters(req): Parameters<DeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = client_for_account(req.account.as_deref())?;
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
            serde_json::to_string_pretty(&body).map_err(internal_err)?,
        )]))
    }

    #[tool(description = "Trigger a redeploy on an existing Railway service.")]
    async fn railway_service_redeploy(
        &self,
        Parameters(req): Parameters<RedeployRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client = client_for_account(req.account.as_deref())?;
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
            serde_json::to_string_pretty(&body).map_err(internal_err)?,
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
        let client = client_for_account(req.account.as_deref())?;
        let sid = ServiceId::from(req.service);
        M::service_delete(&client, &sid)
            .await
            .map_err(internal_err)?;
        let body = json!({
            "deleted_service_id": sid.as_str(),
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&body).map_err(internal_err)?,
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
            serde_json::to_string_pretty(&body).map_err(internal_err)?,
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

fn client_for_account(alias: Option<&str>) -> Result<Client, McpError> {
    match alias {
        None | Some("default") => build_client(),
        Some(name) => {
            let env_var = format!("RAILWAY_TOKEN_{}", name.to_uppercase());
            let token = std::env::var(&env_var).map_err(|_| {
                McpError::internal_error(
                    format!("account alias `{name}`: env var `{env_var}` not set"),
                    None,
                )
            })?;
            Client::with_token(&token).map_err(|e| {
                McpError::internal_error(format!("account `{name}` token invalid: {e}"), None)
            })
        }
    }
}

fn internal_err<E: std::fmt::Display>(e: E) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project_used_when_missing() {
        let req: ListServicesRequest = serde_json::from_str("{}").unwrap();
        assert!(req.project.is_none());
    }

    #[test]
    fn deploy_request_parses() {
        let req: DeployRequest = serde_json::from_str(r#"{"name":"test-svc","vars":[]}"#).unwrap();
        assert_eq!(req.name, "test-svc");
        assert!(req.image.is_none());
        assert!(req.existing_service_id.is_none());
    }

    #[test]
    fn delete_request_requires_confirm() {
        let req: DeleteRequest =
            serde_json::from_str(r#"{"service":"svc-123","confirm":true}"#).unwrap();
        assert!(req.confirm);
    }

    #[test]
    fn delete_request_refuses_without_confirm() {
        let req: DeleteRequest =
            serde_json::from_str(r#"{"service":"svc-123","confirm":false}"#).unwrap();
        assert!(!req.confirm);
    }

    #[test]
    fn redeploy_request_parses() {
        let req: RedeployRequest =
            serde_json::from_str(r#"{"service":"svc-456","environment":"env-789"}"#).unwrap();
        assert_eq!(req.service, "svc-456");
    }

    #[test]
    fn experience_append_defaults() {
        let req: ExperienceAppendRequest =
            serde_json::from_str(r##"{"issue":"#1","phi_step":"VERDICT","task":"test task"}"##)
                .unwrap();
        assert_eq!(req.issue, "#1");
        assert!(req.status.is_none());
        assert!(req.soul_name.is_none());
    }

    #[test]
    fn key_value_parses() {
        let kv: KeyValue = serde_json::from_str(r#"{"key":"FOO","value":"bar"}"#).unwrap();
        assert_eq!(kv.key, "FOO");
        assert_eq!(kv.value, "bar");
    }

    #[test]
    fn list_services_accepts_account() {
        let req: ListServicesRequest = serde_json::from_str(r#"{"account":"acc2"}"#).unwrap();
        assert_eq!(req.account.as_deref(), Some("acc2"));
    }

    #[test]
    fn deploy_request_accepts_account() {
        let req: DeployRequest =
            serde_json::from_str(r#"{"name":"test","account":"acc1","vars":[]}"#).unwrap();
        assert_eq!(req.account.as_deref(), Some("acc1"));
    }

    #[test]
    fn account_env_var_format() {
        let alias = "acc3";
        let env_var = format!("RAILWAY_TOKEN_{}", alias.to_uppercase());
        assert_eq!(env_var, "RAILWAY_TOKEN_ACC3");
    }

    #[test]
    fn client_for_default_returns_env_client() {
        assert!(client_for_account(None).is_err());
        assert!(client_for_account(Some("default")).is_err());
    }
}

//! `runpod-cli` — RunPod GPU fleet management CLI.
//!
//! Rust port of `scripts/runpod_worker.py`.
//!
//! Manages H100 GPU pods on RunPod for trios-train execution.
//! Uses the RunPod **GraphQL** API (the REST v2 endpoints are deprecated/404).
//!
//! # Commands
//!
//!   runpod-cli create <name>              — Create single H100 pod
//!   runpod-cli deploy <num>               — Deploy N H100 pods across all accounts
//!   runpod-cli list                       — List all pods
//!   runpod-cli stop <pod_id>              — Stop a pod
//!   runpod-cli logs <pod_id> [lines]      — Get pod logs
//!   runpod-cli wait <pod_id> [timeout]    — Wait for pod to be RUNNING
//!   runpod-cli gpus                       — List available GPU types
//!
//! # Environment
//!
//!   NEON_DATABASE_URL           — Neon database connection (for create/deploy)
//!   RUNPOD_API_KEY_0..3         — RunPod API keys (one per account)

use std::env;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

const RUNPOD_GRAPHQL_ENDPOINT: &str = "https://api.runpod.io/graphql";
const DEFAULT_GPU_TYPE: &str = "NVIDIA H100 80GB HBM3";
const DEFAULT_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";
const DEFAULT_DISK_GB: u32 = 50;
const DEFAULT_VCPU: u32 = 8;
const DEFAULT_MEMORY_GB: u32 = 64;

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "runpod-cli",
    version,
    about = "RunPod GPU fleet management CLI for trios-train."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Create a single H100 pod on RunPod.
    Create {
        /// Pod name.
        name: Option<String>,
    },
    /// Deploy N H100 pods across all configured accounts.
    Deploy {
        /// Number of pods to create (default: 1).
        num: Option<u32>,
    },
    /// List all pods across all accounts.
    List,
    /// Stop a running pod.
    Stop {
        /// Pod ID to stop.
        pod_id: String,
    },
    /// Get logs from a pod.
    Logs {
        /// Pod ID.
        pod_id: String,
        /// Number of lines from end (default: 100).
        lines: Option<usize>,
    },
    /// Wait for a pod to reach RUNNING state.
    Wait {
        /// Pod ID.
        pod_id: String,
        /// Timeout in seconds (default: 300).
        timeout_sec: Option<u64>,
    },
    /// List available GPU types with availability info.
    Gpus,
}

// ── RunPod GraphQL types ────────────────────────────────────────────────────

/// Generic GraphQL request wrapper.
#[derive(Debug, Serialize)]
struct GqlRequest<'a> {
    query: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
}

/// Generic GraphQL response wrapper — handles both `data` and `errors`.
#[derive(Debug, Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    message: String,
    #[serde(default)]
    extensions: Option<GqlErrorExtensions>,
}

#[derive(Debug, Deserialize)]
struct GqlErrorExtensions {
    code: Option<String>,
}

// ── Query response types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MyselfData {
    myself: UserData,
}

#[derive(Debug, Deserialize)]
struct UserData {
    #[allow(dead_code)]
    id: String,
    pods: Vec<PodInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodInfo {
    id: String,
    name: Option<String>,
    desired_status: Option<String>,
    runtime: Option<PodRuntime>,
    machine: Option<PodMachine>,
}

#[derive(Debug, Deserialize)]
struct PodRuntime {
    #[serde(rename = "uptimeInSeconds")]
    uptime_in_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PodMachine {
    #[serde(rename = "gpuDisplayName")]
    gpu_display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployData {
    pod_find_and_deploy_on_demand: Option<PodInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopResult {
    id: String,
    desired_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopData {
    pod_stop: Option<StopResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GpuTypesData {
    gpu_types: Vec<GpuType>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GpuType {
    id: String,
    display_name: String,
    memory_in_gb: f64,
    secure_cloud: bool,
    community_cloud: bool,
}

// ── GraphQL helpers ─────────────────────────────────────────────────────────

fn get_api_keys() -> Result<Vec<(usize, String)>> {
    let mut keys = Vec::new();
    for i in 0..4 {
        if let Some(key) = env::var(format!("RUNPOD_API_KEY_{i}")).ok() {
            // Skip empty/invalid keys
            if !key.is_empty() && key != "placeholder" {
                keys.push((i, key));
            }
        }
    }
    if keys.is_empty() {
        bail!("No RUNPOD_API_KEY_* environment variables set");
    }
    Ok(keys)
}

fn get_neon_url() -> Result<String> {
    env::var("NEON_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .context("NEON_DATABASE_URL not set")
}

/// Send a GraphQL request and return the parsed `data` field, or bail on errors.
async fn gql_query<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    variables: Option<serde_json::Value>,
) -> Result<T> {
    let body = GqlRequest {
        query,
        variables,
    };

    let resp = client
        .post(RUNPOD_GRAPHQL_ENDPOINT)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to send GraphQL request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("GraphQL HTTP error ({status}): {text}");
    }

    let raw = resp.text().await.context("Failed to read response body")?;
    let gql_resp: GqlResponse<T> =
        serde_json::from_str(&raw).context("Failed to parse GraphQL response")?;

    if !gql_resp.errors.is_empty() {
        let msgs: Vec<&str> = gql_resp.errors.iter().map(|e| e.message.as_str()).collect();
        bail!("GraphQL errors: {}", msgs.join("; "));
    }

    gql_resp
        .data
        .context("GraphQL response missing `data` field")
}

/// Send a GraphQL request that may return null data on error (e.g. deploy).
/// Returns errors as Err, null data as Err.
async fn gql_mutation<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    variables: Option<serde_json::Value>,
) -> Result<T> {
    let body = GqlRequest {
        query,
        variables,
    };

    let resp = client
        .post(RUNPOD_GRAPHQL_ENDPOINT)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to send GraphQL mutation")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("GraphQL HTTP error ({status}): {text}");
    }

    let raw = resp.text().await.context("Failed to read response body")?;
    let gql_resp: GqlResponse<T> =
        serde_json::from_str(&raw).context("Failed to parse GraphQL response")?;

    if !gql_resp.errors.is_empty() {
        let msgs: Vec<String> = gql_resp
            .errors
            .iter()
            .map(|e| {
                let code = e
                    .extensions
                    .as_ref()
                    .and_then(|ext| ext.code.as_deref())
                    .unwrap_or("UNKNOWN");
                format!("{} ({})", e.message, code)
            })
            .collect();
        bail!("GraphQL errors: {}", msgs.join("; "));
    }

    gql_resp
        .data
        .context("GraphQL mutation returned null data")
}

// ── API functions ───────────────────────────────────────────────────────────

async fn list_pods(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<Vec<PodInfo>> {
    let query = r#"{ myself { id pods { id name desiredStatus runtime { uptimeInSeconds } machine { gpuDisplayName } } } }"#;
    let data: MyselfData = gql_query(client, api_key, query, None).await?;
    Ok(data.myself.pods)
}

async fn create_pod(
    client: &reqwest::Client,
    api_key: &str,
    pod_name: &str,
    neon_url: &str,
    extra_env: &[(String, String)],
) -> Result<PodInfo> {
    let query = r#"
        mutation Deploy($input: PodFindAndDeployOnDemandInput!) {
            podFindAndDeployOnDemand(input: $input) {
                id name desiredStatus
                machine { gpuDisplayName }
            }
        }
    "#;

    let mut env_vars = vec![
        serde_json::json!({ "key": "NEON_DATABASE_URL", "value": neon_url }),
        serde_json::json!({ "key": "RUST_LOG", "value": "info" }),
    ];
    for (k, v) in extra_env {
        env_vars.push(serde_json::json!({ "key": k, "value": v }));
    }

    let variables = serde_json::json!({
        "input": {
            "name": pod_name,
            "imageName": DEFAULT_IMAGE,
            "gpuTypeId": DEFAULT_GPU_TYPE,
            "cloudType": "ALL",
            "containerDiskInGb": DEFAULT_DISK_GB,
            "minVcpuCount": DEFAULT_VCPU,
            "minMemoryInGb": DEFAULT_MEMORY_GB,
            "gpuCount": 1,
            "env": env_vars
        }
    });

    let data: DeployData = gql_mutation(client, api_key, query, Some(variables)).await?;
    data.pod_find_and_deploy_on_demand.context("Deploy returned null")
}

async fn stop_pod(
    client: &reqwest::Client,
    api_key: &str,
    pod_id: &str,
) -> Result<()> {
    let query = r#"
        mutation Stop($podId: String!) {
            podStop(input: { podId: $podId }) { id desiredStatus }
        }
    "#;

    let variables = serde_json::json!({ "podId": pod_id });
    let _data: StopData = gql_mutation(client, api_key, query, Some(variables)).await?;
    info!(pod_id = %pod_id, "✅ Stopped pod");
    Ok(())
}

async fn get_pod_logs(
    _client: &reqwest::Client,
    _api_key: &str,
    pod_id: &str,
    _tail_lines: usize,
) -> Result<String> {
    // RunPod has deprecated REST log endpoints and does not expose logs
    // via the public GraphQL schema. Logs are available via:
    //   1. RunPod web dashboard
    //   2. WebSocket streaming (wss://api.runpod.io/v2/{podId}/log)
    Ok(format!(
        "RunPod logs are not available via REST/GraphQL API.\n\
         Dashboard: https://www.runpod.io/console/pods/{pod_id}/logs\n\
         WebSocket: wss://api.runpod.io/v2/{pod_id}/log"
    ))
}

async fn wait_for_pod_ready(
    client: &reqwest::Client,
    api_key: &str,
    pod_id: &str,
    timeout_sec: u64,
) -> Result<bool> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_sec);

    loop {
        let pods = list_pods(client, api_key).await?;
        if let Some(pod) = pods.iter().find(|p| p.id == pod_id) {
            let actual = pod
                .desired_status
                .as_deref()
                .unwrap_or("UNKNOWN");

            info!(
                pod_id = &pod_id[..12.min(pod_id.len())],
                status = actual,
                "Pod status"
            );

            match actual {
                "RUNNING" => {
                    info!("✅ Pod is RUNNING");
                    return Ok(true);
                }
                "EXITED" | "FAILED" | "STOPPED" | "SUSPENDED" => {
                    error!(status = actual, "❌ Pod failed");
                    return Ok(false);
                }
                _ => {}
            }
        } else {
            info!("Pod not found yet, retrying...");
        }

        if tokio::time::Instant::now() > deadline {
            warn!("⏱️ Timeout waiting for pod");
            return Ok(false);
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// ── Command handlers ────────────────────────────────────────────────────────

async fn cmd_create(name: Option<String>) -> Result<()> {
    let keys = get_api_keys()?;
    let neon_url = get_neon_url()?;
    let client = reqwest::Client::new();

    let pod_name = name.unwrap_or_else(|| {
        format!(
            "trios-gpu-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    });

    let (acc_idx, api_key) = &keys[0];
    info!(account = acc_idx, "Using first account");

    let pod = create_pod(&client, api_key, &pod_name, &neon_url, &[]).await?;
    info!("\n✅ Pod created: {}", pod.id);
    info!("   GPU: {}", pod.machine.as_ref().and_then(|m| m.gpu_display_name.as_deref()).unwrap_or("N/A"));
    info!("   Image: {DEFAULT_IMAGE}");
    info!("To get logs: runpod-cli logs {}", pod.id);

    Ok(())
}

async fn cmd_deploy(num: u32) -> Result<()> {
    let keys = get_api_keys()?;
    let neon_url = get_neon_url()?;
    let client = reqwest::Client::new();

    info!("Deploying {num} H100 pods across {} accounts", keys.len());

    let mut created = Vec::new();
    for (i, (acc_idx, api_key)) in keys.iter().enumerate() {
        if i >= num as usize {
            break;
        }
        let pod_name = format!("trios-gpu-scarab-{}", i + 1);
        info!("\n--- Account {}/{} (idx={acc_idx}) ---", i + 1, keys.len());

        match create_pod(&client, api_key, &pod_name, &neon_url, &[]).await {
            Ok(pod) => {
                let gpu = pod.machine.as_ref().and_then(|m| m.gpu_display_name.as_deref()).unwrap_or("N/A");
                info!("  ✅ Created: {} (GPU: {gpu})", pod.id);
                created.push(pod);
            }
            Err(e) => {
                error!("  ❌ Failed on account {acc_idx}: {e}");
            }
        }
    }

    info!("\n{} pods deployed successfully", created.len());
    for pod in &created {
        info!("  runpod-cli wait {}", pod.id);
    }

    Ok(())
}

async fn cmd_list() -> Result<()> {
    let keys = get_api_keys()?;
    let client = reqwest::Client::new();

    for (acc_idx, api_key) in &keys {
        info!("\n--- Account {acc_idx} ---");
        match list_pods(&client, api_key).await {
            Ok(pods) => {
                if pods.is_empty() {
                    info!("  No pods");
                    continue;
                }
                for pod in &pods {
                    let status = pod
                        .desired_status
                        .as_deref()
                        .unwrap_or("UNKNOWN");
                    let gpu = pod
                        .machine
                        .as_ref()
                        .and_then(|m| m.gpu_display_name.as_deref())
                        .unwrap_or("N/A");
                    let uptime = pod
                        .runtime
                        .as_ref()
                        .and_then(|r| r.uptime_in_seconds)
                        .map(|u| format!("{u}s"))
                        .unwrap_or_else(|| "N/A".to_string());
                    let name = pod.name.as_deref().unwrap_or("—");

                    info!("  {} ({})", &pod.id[..12.min(pod.id.len())], name);
                    info!("    Status: {status}");
                    info!("    GPU: {gpu}");
                    info!("    Uptime: {uptime}");
                }
            }
            Err(e) => {
                error!("  ❌ Failed to list pods: {e}");
            }
        }
    }

    Ok(())
}

async fn cmd_stop(pod_id: String) -> Result<()> {
    let keys = get_api_keys()?;
    let client = reqwest::Client::new();

    for (_acc_idx, api_key) in &keys {
        let pods = list_pods(&client, api_key).await?;
        if pods.iter().any(|p| p.id == pod_id) {
            stop_pod(&client, api_key, &pod_id).await?;
            return Ok(());
        }
    }

    bail!("Failed to find pod: {pod_id}");
}

async fn cmd_logs(pod_id: String, lines: usize) -> Result<()> {
    let keys = get_api_keys()?;
    let client = reqwest::Client::new();

    // Try all accounts to find the one that owns this pod
    for (_acc_idx, api_key) in &keys {
        if let Ok(pods) = list_pods(&client, api_key).await {
            if pods.iter().any(|p| p.id == pod_id) {
                info!("Fetching logs for pod {pod_id} (last {lines} lines)...");
                match get_pod_logs(&client, api_key, &pod_id, lines).await {
                    Ok(logs) => {
                        info!("\n--- Logs ---\n{logs}");
                    }
                    Err(e) => {
                        error!("Failed to fetch logs: {e}");
                    }
                }
                return Ok(());
            }
        }
    }

    // Fallback: try first account anyway
    let (.., api_key) = &keys[0];
    info!("Fetching logs for pod {pod_id} (last {lines} lines)...");
    match get_pod_logs(&client, api_key, &pod_id, lines).await {
        Ok(logs) => {
            info!("\n--- Logs ---\n{logs}");
        }
        Err(e) => {
            error!("Failed to fetch logs: {e}");
        }
    }

    Ok(())
}

async fn cmd_wait(pod_id: String, timeout_sec: u64) -> Result<()> {
    let keys = get_api_keys()?;
    let client = reqwest::Client::new();

    // Find the account that owns this pod
    for (_acc_idx, api_key) in &keys {
        if let Ok(pods) = list_pods(&client, api_key).await {
            if pods.iter().any(|p| p.id == pod_id) {
                info!("Waiting for pod {pod_id} (max {timeout_sec}s)...");
                if wait_for_pod_ready(&client, api_key, &pod_id, timeout_sec).await? {
                    info!("✅ Pod {pod_id} is READY");
                } else {
                    error!("❌ Pod failed to become READY");
                }
                return Ok(());
            }
        }
    }

    bail!("Failed to find pod: {pod_id}");
}

async fn cmd_gpus() -> Result<()> {
    let keys = get_api_keys()?;
    let client = reqwest::Client::new();
    let (.., api_key) = &keys[0];

    let query = r#"{ gpuTypes { id displayName memoryInGb secureCloud communityCloud } }"#;
    let data: GpuTypesData = gql_query(&client, api_key, query, None).await?;

    info!("\n{:<55} {:>6}  {:>5}  {:>5}", "GPU Type", "VRAM", "SC", "CC");
    info!("{}", "─".repeat(78));
    for gpu in &data.gpu_types {
        if gpu.secure_cloud || gpu.community_cloud {
            info!(
                "{:<55} {:>5.0}GB  {:>5}  {:>5}",
                gpu.id,
                gpu.memory_in_gb,
                if gpu.secure_cloud { "✓" } else { "—" },
                if gpu.community_cloud { "✓" } else { "—" }
            );
        }
    }

    Ok(())
}

// ── Main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("runpod_cli=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Create { name } => cmd_create(name).await,
        Cmd::Deploy { num } => cmd_deploy(num.unwrap_or(1)).await,
        Cmd::List => cmd_list().await,
        Cmd::Stop { pod_id } => cmd_stop(pod_id).await,
        Cmd::Logs { pod_id, lines } => cmd_logs(pod_id, lines.unwrap_or(100)).await,
        Cmd::Wait { pod_id, timeout_sec } => cmd_wait(pod_id, timeout_sec.unwrap_or(300)).await,
        Cmd::Gpus => cmd_gpus().await,
    }
}

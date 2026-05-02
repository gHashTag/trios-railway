//! `fleet-probe` — O(1) parallel health check across all 7 Railway accounts.
//!
//! One HTTPS call per account, fanned out via `tokio::spawn`. Prints a single
//! human-readable table. No file writes, no side effects. Safe to run at any cadence.
//!
//! Usage:
//! ```bash
//! source .railway_creds.env
//! cargo run -p trios-igla-ops --bin fleet-probe
//! ```
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use trios_igla_ops::accounts::{Account, ACCOUNTS};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ServiceNode {
    #[allow(dead_code)]
    id: String,
    name: String,
}
#[derive(Deserialize, Debug)]
struct Edge {
    node: ServiceNode,
}
#[derive(Deserialize, Debug)]
struct Edges {
    edges: Vec<Edge>,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ProjNode {
    name: String,
    services: Option<Edges>,
}
#[derive(Deserialize, Debug)]
struct RespData {
    project: Option<ProjNode>,
}
#[derive(Deserialize, Debug)]
struct GqlResp {
    data: Option<RespData>,
    errors: Option<serde_json::Value>,
}

const Q: &str = "query($id:String!){project(id:$id){name services{edges{node{id name}}}}}";

async fn probe(acc: &'static Account) -> (String, String, String, Vec<String>) {
    let tok = match std::env::var(acc.env_tok) {
        Ok(t) => t,
        Err(_) => {
            return (
                acc.tag.into(),
                "NO_TOKEN".into(),
                format!("env {} unset", acc.env_tok),
                vec![],
            )
        }
    };
    let (h_name, h_val) = acc.kind.auth_header(&tok);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (acc.tag.into(), "CLIENT_ERR".into(), e.to_string(), vec![]),
    };
    let body = json!({"query": Q, "variables": {"id": acc.project}});
    let resp = client
        .post("https://backboard.railway.app/graphql/v2")
        .header("Content-Type", "application/json")
        .header(h_name, h_val)
        .json(&body)
        .send()
        .await;
    let resp = match resp {
        Ok(r) => r,
        Err(e) => return (acc.tag.into(), "HTTP_ERR".into(), e.to_string(), vec![]),
    };
    let gql: GqlResp = match resp.json().await {
        Ok(g) => g,
        Err(e) => return (acc.tag.into(), "PARSE_ERR".into(), e.to_string(), vec![]),
    };
    if let Some(errs) = gql.errors {
        return (acc.tag.into(), "AUTH_ERR".into(), errs.to_string(), vec![]);
    }
    match gql.data.and_then(|d| d.project) {
        Some(p) => {
            let svcs = p
                .services
                .map(|s| s.edges.into_iter().map(|e| e.node.name).collect::<Vec<_>>())
                .unwrap_or_default();
            (acc.tag.into(), "OK".into(), p.name, svcs)
        }
        None => (
            acc.tag.into(),
            "NOT_FOUND".into(),
            "project=null".into(),
            vec![],
        ),
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    let handles: Vec<_> = ACCOUNTS.iter().map(|a| tokio::spawn(probe(a))).collect();
    println!(
        "{:<5} {:<10} {:<7} {:<32} {}",
        "ACC", "STATUS", "LANE", "PROJECT", "SERVICES"
    );
    for (a, h) in ACCOUNTS.iter().zip(handles) {
        let (tag, status, info, svcs) = h.await?;
        let project_name = if svcs.is_empty() {
            info.chars().take(30).collect::<String>()
        } else {
            info
        };
        let svc_str = if svcs.is_empty() {
            "-".into()
        } else {
            format!("{} [{}]", svcs.len(), svcs.join(","))
        };
        let svc_trunc: String = svc_str.chars().take(100).collect();
        println!(
            "{:<5} {:<10} {:<7} {:<32} {}",
            tag,
            status,
            &a.lane[13..].chars().take(6).collect::<String>(),
            project_name.chars().take(30).collect::<String>(),
            svc_trunc
        );
    }
    Ok(())
}

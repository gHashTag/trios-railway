//! In-process tracking of MCP client activity. Operator-friendly summary
//! that surfaces which connectors are calling which tools, against
//! which accounts. Periodically dumped to stdout as JSON so Railway
//! logs capture the activity timeline.
//!
//! **Pure data + small mutex**, no async; called from every tool
//! handler at entry. Aggregation is per-process; restarts wipe.
//!
//! Anchor: `phi^2 + phi^-2 = 3`.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionStats {
    pub client_id: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub tool_counts: BTreeMap<String, u64>,
    pub account_counts: BTreeMap<String, u64>,
}

static STORE: OnceLock<Mutex<BTreeMap<String, ConnectionStats>>> = OnceLock::new();

fn store() -> &'static Mutex<BTreeMap<String, ConnectionStats>> {
    STORE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Record one tool-call. Caller passes `client_id` (best-effort: the
/// `clientInfo.name` from the MCP `initialize` handshake, or `"unknown"`).
pub fn log_call(client_id: &str, tool: &str, account: Option<&str>) {
    let now = Utc::now();
    let mut map = match store().lock() {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(?e, "connections store mutex poisoned");
            return;
        }
    };
    let entry = map
        .entry(client_id.to_string())
        .or_insert_with(|| ConnectionStats {
            client_id: client_id.to_string(),
            first_seen: now,
            last_seen: now,
            tool_counts: BTreeMap::new(),
            account_counts: BTreeMap::new(),
        });
    entry.last_seen = now;
    *entry.tool_counts.entry(tool.to_string()).or_insert(0) += 1;
    if let Some(a) = account {
        *entry.account_counts.entry(a.to_string()).or_insert(0) += 1;
    }
}

/// Snapshot all tracked connections.
pub fn snapshot() -> Vec<ConnectionStats> {
    let map = match store().lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    map.values().cloned().collect()
}

/// Render a one-line JSON summary suitable for stdout logging.
pub fn render_summary_line() -> String {
    let snap = snapshot();
    let payload = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "kind": "connection-summary",
        "connections": snap,
    });
    payload.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_store() {
        if let Some(m) = STORE.get() {
            if let Ok(mut g) = m.lock() {
                g.clear();
            }
        }
    }

    #[test]
    fn log_call_increments_counters() {
        reset_store();
        log_call("client-A", "mcp.railway.list", Some("acc0"));
        log_call("client-A", "mcp.railway.list", Some("acc0"));
        log_call("client-A", "mcp.fleet.snapshot", None);
        let snap = snapshot();
        let entry = snap.iter().find(|s| s.client_id == "client-A").unwrap();
        assert_eq!(entry.tool_counts["mcp.railway.list"], 2);
        assert_eq!(entry.tool_counts["mcp.fleet.snapshot"], 1);
        assert_eq!(entry.account_counts["acc0"], 2);
    }

    #[test]
    fn render_summary_returns_valid_json() {
        reset_store();
        log_call("clientB", "mcp.fleet.snapshot", None);
        let line = render_summary_line();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["kind"], "connection-summary");
        assert!(v["connections"].is_array());
    }

    #[test]
    fn unknown_account_falls_back_to_no_count() {
        reset_store();
        log_call("clientC", "mcp.audit.migrate", None);
        let snap = snapshot();
        let entry = snap.iter().find(|s| s.client_id == "clientC").unwrap();
        assert!(entry.account_counts.is_empty());
        assert_eq!(entry.tool_counts["mcp.audit.migrate"], 1);
    }
}

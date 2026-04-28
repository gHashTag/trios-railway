//! Tripwires #107..#114 — runtime guards enforced at every MCP tool entry.
//!
//! Each guard returns `Result<(), McpError>`; tool handlers call them
//! at the top of their async function so a violation short-circuits
//! before any Railway / Neon mutation. R5-honest: each error returns
//! a typed message; never silent.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`.

use rmcp::ErrorData as McpError;
use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use trios_railway_core::multiclient::assert_project_allowed;

// ---------------------------------------------------------------------
// #107 — Project whitelist (re-exported from trios-railway-core)
// ---------------------------------------------------------------------

pub fn t107_project_whitelist(project: &str) -> Result<(), McpError> {
    assert_project_allowed(project).map_err(|e| {
        McpError::invalid_params(format!("tripwire #107: {e}"), None)
    })
}

// ---------------------------------------------------------------------
// #109 — direct_railway_call_forbidden
// ---------------------------------------------------------------------
//
// MCP tool surface is the only sanctioned path to Railway. We can't see
// the raw HTTP request from inside an `rmcp` tool handler, so we keep
// this tripwire as a marker test that exercises the contract: any code
// path that bypasses `tools.rs` to call `trios_railway_core::Client`
// directly must go through `RailwayMultiClient::get(account)`. That's
// already enforced at the type level (Client construction requires
// either RailwayMultiClient or build_client_for(alias)). The runtime
// check here is a cheap belt-and-braces: callers must pass a non-empty
// `_via_mcp` marker string that the tool handler always supplies; if a
// future rogue caller forgets it, the call fails at parse time.

pub fn t109_no_direct_call(via_mcp_marker: &str) -> Result<(), McpError> {
    if via_mcp_marker == "mcp" {
        Ok(())
    } else {
        Err(McpError::invalid_params(
            "tripwire #109: caller must declare via_mcp_marker='mcp'; \
             direct Railway calls outside the MCP surface are forbidden"
                .to_string(),
            None,
        ))
    }
}

// ---------------------------------------------------------------------
// #110 — audit_ledger_append_only
// ---------------------------------------------------------------------
//
// `mcp.experience.append` writes one line per call. Any caller that
// ships a payload containing UPDATE/DELETE/TRUNCATE on the audit ledger
// is rejected. We pattern-match the SQL-shape strings (case-insensitive)
// so a typo doesn't slip through. R5: error message is explicit.

pub fn t110_audit_append_only(task: &str) -> Result<(), McpError> {
    let needle = task.to_ascii_uppercase();
    for forbidden in ["UPDATE ", "DELETE ", "TRUNCATE ", "DROP TABLE", "ALTER TABLE"] {
        if needle.contains(forbidden) {
            return Err(McpError::invalid_params(
                format!(
                    "tripwire #110: audit ledger is append-only; task contains forbidden verb {forbidden:?}"
                ),
                None,
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// #111 — mcp_tool_signature_match
// ---------------------------------------------------------------------
//
// Every tool name we expose must match either the legacy `railway_*`
// allowlist or the new `mcp.<domain>.<verb>` shape. Catches typos at
// dispatch time before the call hits Railway.

const LEGACY_NAMES: &[&str] = &[
    "railway_service_list",
    "railway_service_deploy",
    "railway_service_redeploy",
    "railway_service_delete",
    "railway_experience_append",
    "railway_audit_migrate_sql",
];

pub fn t111_tool_signature(name: &str) -> Result<(), McpError> {
    if LEGACY_NAMES.contains(&name) {
        return Ok(());
    }
    // mcp.<domain>.<verb> — exactly two dots, three lowercase tokens.
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() == 3
        && parts[0] == "mcp"
        && !parts[1].is_empty()
        && !parts[2].is_empty()
        && parts.iter().all(|p| {
            p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
    {
        Ok(())
    } else {
        Err(McpError::invalid_params(
            format!(
                "tripwire #111: tool name {name:?} matches neither legacy `railway_*` nor `mcp.<domain>.<verb>` shape"
            ),
            None,
        ))
    }
}

// ---------------------------------------------------------------------
// #112 — mcp_account_scoped
// ---------------------------------------------------------------------
//
// Any *write* tool (deploy, redeploy, delete, cleanup) requires an
// explicit `account` argument. Read-only tools may omit it (and then
// fall back to the default RAILWAY_TOKEN). This protects against
// "wrong fleet" mutations when an operator forgets the alias.

pub fn t112_account_scoped(account: Option<&str>) -> Result<(), McpError> {
    match account {
        Some(s) if !s.is_empty() => Ok(()),
        _ => Err(McpError::invalid_params(
            "tripwire #112: write tools require explicit account=acc0/acc1/acc2/acc3"
                .to_string(),
            None,
        )),
    }
}

// ---------------------------------------------------------------------
// #113 — mcp_dry_run_default
// ---------------------------------------------------------------------
//
// Destructive tools (delete, cleanup) must come with either:
//   * `dry_run = true`  (preview only)  OR
//   * `dry_run = false` AND `confirm = true`
// Anything else is rejected — we never let a default-confirmed call go
// through.

pub fn t113_dry_run_default(confirm: bool, dry_run: Option<bool>) -> Result<(), McpError> {
    let dry = dry_run.unwrap_or(true);
    if dry {
        // dry-run is always safe regardless of confirm
        return Ok(());
    }
    if !confirm {
        return Err(McpError::invalid_params(
            "tripwire #113: destructive op needs `dry_run=false` AND `confirm=true`; \
             default policy is dry-run"
                .to_string(),
            None,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// #114 — mcp_idempotency_key
// ---------------------------------------------------------------------
//
// Idempotent operations (e.g. deploy of an existing service name) must
// carry an idempotency key. We track the (key, tool) pairs seen this
// process and reject duplicates: same key + same tool + within window
// returns the cached "already done" outcome instead of re-applying.
//
// The cache is per-process; restarts reset it (which is fine — Railway
// itself is the source of truth for "exists?").

static SEEN_KEYS: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();

fn seen() -> &'static Mutex<BTreeSet<String>> {
    SEEN_KEYS.get_or_init(|| Mutex::new(BTreeSet::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyOutcome {
    /// First time we've seen this key — proceed with the operation.
    First,
    /// We've already executed this key+tool — return cached success.
    Replay,
}

pub fn t114_idempotency_key(
    tool: &str,
    key: Option<&str>,
) -> Result<IdempotencyOutcome, McpError> {
    let key = key.ok_or_else(|| {
        McpError::invalid_params(
            "tripwire #114: idempotent ops require explicit `idempotency_key`".to_string(),
            None,
        )
    })?;
    let composite = format!("{tool}::{key}");
    let mut set = seen().lock().map_err(|e| {
        McpError::internal_error(format!("idempotency lock poisoned: {e}"), None)
    })?;
    if set.contains(&composite) {
        return Ok(IdempotencyOutcome::Replay);
    }
    set.insert(composite);
    Ok(IdempotencyOutcome::First)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t107_accepts_known_project() {
        assert!(t107_project_whitelist("e4fe33bb-3b09-4842-9782-7d2dea1abc9b").is_ok());
    }

    #[test]
    fn t107_rejects_unknown_project() {
        let err = t107_project_whitelist("00000000-0000-0000-0000-000000000000");
        assert!(err.is_err());
    }

    #[test]
    fn t109_accepts_mcp_marker() {
        assert!(t109_no_direct_call("mcp").is_ok());
    }

    #[test]
    fn t109_rejects_anything_else() {
        assert!(t109_no_direct_call("").is_err());
        assert!(t109_no_direct_call("curl").is_err());
    }

    #[test]
    fn t110_accepts_normal_task() {
        assert!(t110_audit_append_only("mcp deploy IGLA-TRAIN_V2-FP32-E0042-rng43").is_ok());
    }

    #[test]
    fn t110_rejects_update_in_task() {
        assert!(t110_audit_append_only("UPDATE audit_ledger SET ...").is_err());
        assert!(t110_audit_append_only("delete from gardener_runs").is_err());
        assert!(t110_audit_append_only("DROP TABLE bpb_samples").is_err());
    }

    #[test]
    fn t111_accepts_legacy_names() {
        for n in LEGACY_NAMES {
            assert!(t111_tool_signature(n).is_ok(), "{n} should be accepted");
        }
    }

    #[test]
    fn t111_accepts_new_mcp_dot_names() {
        for n in [
            "mcp.railway.list",
            "mcp.railway.deploy",
            "mcp.railway.redeploy",
            "mcp.railway.delete",
            "mcp.experience.append",
            "mcp.audit.migrate",
            "mcp.fleet.snapshot",
            "mcp.fleet.cleanup",
            "mcp.igla.validate",
        ] {
            assert!(t111_tool_signature(n).is_ok(), "{n} should be accepted");
        }
    }

    #[test]
    fn t111_rejects_unknown_shape() {
        for n in ["mcp.railway", "MCP.RAILWAY.LIST", "foo.bar.baz", "mcp..deploy"] {
            assert!(t111_tool_signature(n).is_err(), "{n} should be rejected");
        }
    }

    #[test]
    fn t112_accepts_known_account() {
        assert!(t112_account_scoped(Some("acc0")).is_ok());
        assert!(t112_account_scoped(Some("acc3")).is_ok());
    }

    #[test]
    fn t112_rejects_missing_or_empty_account() {
        assert!(t112_account_scoped(None).is_err());
        assert!(t112_account_scoped(Some("")).is_err());
    }

    #[test]
    fn t113_dry_run_default_is_safe() {
        assert!(t113_dry_run_default(false, None).is_ok());
    }

    #[test]
    fn t113_explicit_dry_run_false_needs_confirm() {
        assert!(t113_dry_run_default(false, Some(false)).is_err());
        assert!(t113_dry_run_default(true, Some(false)).is_ok());
    }

    #[test]
    fn t113_dry_run_true_ignores_confirm() {
        assert!(t113_dry_run_default(false, Some(true)).is_ok());
        assert!(t113_dry_run_default(true, Some(true)).is_ok());
    }

    #[test]
    fn t114_first_then_replay_for_same_key() {
        let key = "test-key-unique-1";
        let out1 = t114_idempotency_key("mcp.railway.deploy", Some(key)).unwrap();
        assert_eq!(out1, IdempotencyOutcome::First);
        let out2 = t114_idempotency_key("mcp.railway.deploy", Some(key)).unwrap();
        assert_eq!(out2, IdempotencyOutcome::Replay);
    }

    #[test]
    fn t114_rejects_missing_key() {
        let err = t114_idempotency_key("mcp.railway.deploy", None);
        assert!(err.is_err());
    }
}

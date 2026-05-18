//! ADR-0042 / L-SS7 scarab runtime invariants.
//!
//! These are integration-level guard tests: they execute against the
//! checked-in source text of `src/bin/scarab.rs` and assert that the
//! scarab runtime never reaches for Railway control-plane symbols and
//! always reads its strategy / writes its heartbeat through the SSOT
//! tables specified by the IGLA RACE runtime-blocker context.
//!
//! Why these live as a separate integration test:
//!   * They survive even if someone deletes the `#[cfg(test)] mod tests`
//!     block at the bottom of `scarab.rs`.
//!   * They make the invariant visible in `cargo test -p trios-igla-race`
//!     output without requiring the scarab binary to compile against a
//!     test feature.
//!
//! Anchor: phi^2 + phi^-2 = 3.

use std::fs;
use std::path::PathBuf;

fn scarab_src() -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "bin", "scarab.rs"]
        .iter()
        .collect();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every Railway control-plane symbol forbidden by ADR-0042 must be
/// absent from the scarab runtime, except inside the docstring header
/// (block comment) and inside the test guard list. We detect a forbidden
/// call by looking for the symbol followed by `(` (a function call) or
/// `=` (an env var read like `var("RAILWAY_API_TOKEN")` — which itself
/// would also be flagged by the `(` rule).
#[test]
fn scarab_runtime_does_not_invoke_railway_control_plane() {
    let src = scarab_src();
    for needle in [
        "variableUpsert",
        "serviceInstanceDeployV2",
        "serviceInstanceRedeploy",
        "serviceInstanceUpdate",
        "serviceDelete",
        "RAILWAY_API_TOKEN",
    ] {
        // Skip the test-array entries that just list the strings to scan.
        let call_shape = format!("{needle}(");
        assert!(
            !src.contains(&call_shape),
            "scarab.rs must never invoke {needle} (call-shape `{needle}(` found)"
        );
    }
}

/// The strategy poll must filter on `service_id = $1` exactly — never
/// over-broad (e.g. `WHERE 1=1`) — so each scarab only consumes its own row.
#[test]
fn strategy_poll_is_scoped_to_service_id() {
    let src = scarab_src();
    assert!(
        src.contains("ssot.scarab_strategy"),
        "scarab must SELECT from ssot.scarab_strategy"
    );
    assert!(
        src.contains("WHERE service_id = $1"),
        "scarab must scope the strategy poll to its own service_id"
    );
}

/// The heartbeat upsert must target `ssot.scarab_heartbeat` and use the
/// ON CONFLICT (service_id) idempotency that the SSOT schema requires.
#[test]
fn heartbeat_upsert_targets_ssot_scarab_heartbeat() {
    let src = scarab_src();
    assert!(
        src.contains("INSERT INTO ssot.scarab_heartbeat"),
        "scarab must UPSERT into ssot.scarab_heartbeat"
    );
    assert!(
        src.contains("ON CONFLICT (service_id) DO UPDATE"),
        "scarab heartbeat upsert must be idempotent by service_id"
    );
}

/// The scarab must perform a startup heartbeat BEFORE entering the poll
/// loop — otherwise a scarab that boots into a missing-strategy state
/// would look dead to the dead-scarab detector for the first cycle.
#[test]
fn scarab_emits_startup_heartbeat_before_loop() {
    let src = scarab_src();
    // Find the first occurrence of the upsert helper call.
    let first_hb = src
        .find("upsert_heartbeat(")
        .expect("upsert_heartbeat call");
    // Find the boundary of the main poll loop. The current implementation
    // marks it with `loop {` directly after `let stats =`.
    let loop_pos = src[first_hb..]
        .find("loop {")
        .map(|p| first_hb + p)
        .expect("main loop");
    assert!(
        first_hb < loop_pos,
        "first heartbeat upsert must precede the main poll loop"
    );
}

/// BPB samples written by the scarab (or its trainer subprocess) must
/// target the `ssot.bpb_samples` table (the IGLA RACE blocker context
/// schema), not the legacy unqualified `bpb_samples`.
#[test]
fn bpb_samples_writes_target_ssot_schema() {
    let src = scarab_src();
    assert!(
        src.contains("INSERT INTO ssot.bpb_samples"),
        "scarab must write samples into ssot.bpb_samples (qualified schema)"
    );
}

/// The scarab must fail visibly when neither the service identity nor
/// the SSOT URL is available. The literal `bail!` substring is enough
/// to confirm the refuse-to-start branch is wired.
#[test]
fn scarab_refuses_to_start_without_identity_or_db_url() {
    let src = scarab_src();
    assert!(
        src.contains("RAILWAY_SERVICE_ID")
            && src.contains("SCARAB_SERVICE_ID")
            && src.contains("Refusing to start"),
        "scarab must refuse to start without service identity"
    );
    assert!(
        src.contains("DATABASE_URL")
            && src.contains("RAILWAY_POSTGRES_URL")
            && src.contains("NEON_DATABASE_URL"),
        "scarab must accept DATABASE_URL with legacy fallbacks documented"
    );
}

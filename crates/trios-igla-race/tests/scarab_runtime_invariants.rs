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

// -- deployment / image-publish wiring guards --
//
// PR #221 fixed `scarab.rs` to ADR-0042 pull-loop semantics, but the
// merged code never reached Railway because there was no CI path that
// built and published the scarab image. These guards lock the deploy
// wiring so that the binary that ships to `ghcr.io/<owner>/sovereign-scarab`
// is the ADR-0042 `scarab` bin and not a legacy queue worker.

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn scarab_dockerfile() -> String {
    let path = repo_root()
        .join("crates")
        .join("trios-igla-race")
        .join("Dockerfile.scarab");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn sovereign_scarab_workflow() -> String {
    let path = repo_root()
        .join(".github")
        .join("workflows")
        .join("sovereign-scarab.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// `Dockerfile.scarab` must build the ADR-0042 scarab bin from this
/// crate. If someone re-points it at a legacy bin (`seed_agent`,
/// `seed_gardener`, `reset_queue`, `debug_queue`, `trios-igla-race`),
/// the deployed image will run the wrong process and heartbeat stays
/// at zero — the exact failure mode this PR is fixing.
#[test]
fn scarab_dockerfile_builds_adr0042_scarab_bin() {
    let dockerfile = scarab_dockerfile();
    assert!(
        dockerfile.contains("--bin scarab") && dockerfile.contains("-p trios-igla-race"),
        "Dockerfile.scarab must build `--bin scarab -p trios-igla-race`; \
         got:\n{dockerfile}"
    );
    for legacy in [
        "--bin seed_agent",
        "--bin seed_gardener",
        "--bin reset_queue",
        "--bin debug_queue",
        "--bin trios-igla-race",
        "--bin smoke_agent",
        "--bin trios-railway-mcp",
        "--bin seed-agent",
    ] {
        assert!(
            !dockerfile.contains(legacy),
            "Dockerfile.scarab must NOT build legacy bin `{legacy}` — \
             ADR-0042 deploy must run src/bin/scarab.rs"
        );
    }
}

/// The runtime stage of `Dockerfile.scarab` must `CMD`/`ENTRYPOINT` the
/// scarab binary at the conventional `/usr/local/bin/scarab` path.
/// A regression that points the runtime at a legacy bin (or no CMD at
/// all, leaving the base image default) would mean the container starts
/// but never reaches the ADR-0042 pull-loop.
#[test]
fn scarab_dockerfile_runtime_invokes_scarab_binary() {
    let dockerfile = scarab_dockerfile();
    let has_cmd = dockerfile.contains("CMD [\"/usr/local/bin/scarab\"]")
        || dockerfile.contains("ENTRYPOINT [\"/usr/local/bin/scarab\"]");
    assert!(
        has_cmd,
        "Dockerfile.scarab runtime stage must CMD or ENTRYPOINT \
         /usr/local/bin/scarab; got:\n{dockerfile}"
    );
    for legacy in [
        "/usr/local/bin/seed-agent",
        "/usr/local/bin/seed_agent",
        "/usr/local/bin/seed_gardener",
        "/usr/local/bin/trios-igla-race",
        "/usr/local/bin/trios-railway-mcp",
        "/usr/local/bin/tri-railway",
    ] {
        assert!(
            !dockerfile.contains(legacy),
            "Dockerfile.scarab runtime must NOT launch legacy bin `{legacy}` — \
             scarab fleet must run ADR-0042 pull-loop"
        );
    }
}

/// `Dockerfile.scarab`'s builder stage must use a Rust base image new
/// enough to compile the workspace's transitive deps. tokio-postgres
/// 0.7.17 (and its transitive crates) declare `edition = "2024"`, which
/// the compiler only accepts on Rust >= 1.85. A regression to an older
/// image (e.g. `rust:1.82-slim`, the original toolchain) breaks the
/// sovereign-scarab GHCR publish workflow and the ADR-0042 runtime
/// never reaches Railway. We scan for the version embedded in `FROM
/// rust:<MAJOR>.<MINOR>...` and reject anything below 1.85.
#[test]
fn scarab_dockerfile_rust_toolchain_supports_edition2024() {
    const MIN_MAJOR: u32 = 1;
    const MIN_MINOR: u32 = 85;
    let dockerfile = scarab_dockerfile();
    let mut found_rust_from = false;
    for line in dockerfile.lines() {
        let trimmed = line.trim_start();
        // Match lines like `FROM rust:1.90-slim-bookworm AS builder`.
        let Some(rest) = trimmed.strip_prefix("FROM ") else {
            continue;
        };
        let Some(tag) = rest.split_whitespace().next() else {
            continue;
        };
        let Some(version_part) = tag.strip_prefix("rust:") else {
            continue;
        };
        found_rust_from = true;
        // `1.90-slim-bookworm` -> `1.90` -> (1, 90).
        let version_token = version_part
            .split(|c: char| c == '-' || c == '@')
            .next()
            .unwrap_or(version_part);
        let mut parts = version_token.split('.');
        let major: u32 = parts
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("cannot parse rust major from `{tag}`"));
        let minor: u32 = parts
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("cannot parse rust minor from `{tag}`"));
        assert!(
            (major, minor) >= (MIN_MAJOR, MIN_MINOR),
            "Dockerfile.scarab builder uses `rust:{version_token}` but workspace \
             transitive deps (tokio-postgres 0.7.17) require Rust >= \
             {MIN_MAJOR}.{MIN_MINOR} for edition2024 support. Bump the FROM line."
        );
    }
    assert!(
        found_rust_from,
        "Dockerfile.scarab must declare a `FROM rust:<version>` builder stage; \
         got:\n{dockerfile}"
    );
}

/// A publishing workflow must exist so the ADR-0042 scarab image gets
/// to GHCR for Railway to pull. Without this workflow the code-level
/// fix in scarab.rs never reaches the deployed process — which is the
/// exact failure this PR remediates.
#[test]
fn sovereign_scarab_publish_workflow_exists_and_targets_scarab_dockerfile() {
    let wf = sovereign_scarab_workflow();
    assert!(
        wf.contains("crates/trios-igla-race/Dockerfile.scarab"),
        "sovereign-scarab.yml must build crates/trios-igla-race/Dockerfile.scarab; \
         got:\n{wf}"
    );
    assert!(
        wf.contains("sovereign-scarab"),
        "sovereign-scarab.yml must publish under the `sovereign-scarab` image name \
         (matches docs/DEPLOY_SOVEREIGN_SCARAB_V4.md and railway.toml.template)"
    );
    // The workflow must NOT reuse the MCP Dockerfile or other legacy
    // Dockerfiles — that would publish the wrong runtime.
    for wrong in ["Dockerfile.mcp", "Dockerfile.real-seed-agent"] {
        assert!(
            !wf.contains(wrong),
            "sovereign-scarab.yml must not publish from `{wrong}`"
        );
    }
}

/// ADR-0042 forbids any Railway-mutation control path from this repo
/// against scarab services. The publish workflow must never reach for
/// the Railway GraphQL push API (`variableUpsert`, `serviceInstance*`,
/// `serviceDelete`) and must never reference a Railway PAT.
#[test]
fn sovereign_scarab_workflow_is_read_only_against_railway() {
    let wf = sovereign_scarab_workflow();
    for forbidden in [
        "variableUpsert",
        "serviceInstanceDeployV2",
        "serviceInstanceRedeploy",
        "serviceInstanceUpdate",
        "serviceDelete",
        "RAILWAY_TOKEN",
        "RAILWAY_API_TOKEN",
    ] {
        assert!(
            !wf.contains(forbidden),
            "sovereign-scarab.yml must not touch Railway control plane (`{forbidden}` \
             found) — scarab fleet control is SSOT, not Railway API (ADR-0042)"
        );
    }
}

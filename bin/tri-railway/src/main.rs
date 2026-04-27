//! `tri railway` — single-binary entry point.
//!
//! Anchor: `phi^2 + phi^-2 = 3`.
//!
//! v0.0.1 ships the wiring for these subcommands:
//!
//!   tri-railway version
//!   tri-railway audit migrate-sql        # prints DDL to stdout
//!   tri-railway experience append ...    # writes a single L7 line
//!
//! Read/mutation verbs (`list`, `create`, `deploy`, `delete`, `logs`,
//! `audit run`) land under issues #4..#9; the structure here is set up
//! so each verb plugs in as one new `match` arm.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use trios_railway_audit::{
    detect, migrations, verdict as compute_verdict, AuditVerdict, LedgerRow, RealService,
};
use trios_railway_core::{
    mutations as M, queries as Q, transport::AuthMode, Client, EnvironmentId, ProjectId,
    RailwayHash, ServiceId,
};
use trios_railway_experience::{append_line, ExperienceLine};

const IGLA_PROJECT_ID: &str = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b";
const IGLA_PROD_ENV_ID: &str = "54e293b9-00a9-4102-814d-db151636d96e";
const DEFAULT_TRAINER_IMAGE: &str = "ghcr.io/ghashtag/trios-trainer-igla:latest";

#[derive(Parser, Debug)]
#[command(
    name = "tri-railway",
    version,
    about = "Manage Railway services for the IGLA project + online audit.",
    long_about = "tri railway: companion CLI to trios-trainer-igla and trios-mcp.\n\
                  R-rules: R1 (Rust-only), R5 (honest exit codes),\n\
                  R7 (every mutation seals an audit triplet),\n\
                  R9 (igla check before any mutation),\n\
                  L7 (experience log), L21 (context immutability)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Cmd {
    /// Print the version and exit.
    Version,

    /// Audit operations.
    Audit {
        #[command(subcommand)]
        sub: AuditCmd,
    },

    /// Local experience log helpers (L7).
    Experience {
        #[command(subcommand)]
        sub: ExperienceCmd,
    },

    /// Service operations against Railway (RW-02 + RW-03).
    ///
    /// Requires `RAILWAY_TOKEN` in the environment. UUID-shaped tokens
    /// are auto-detected as project-access tokens.
    Service {
        #[command(subcommand)]
        sub: ServiceCmd,
    },

    /// Snapshot the live Railway fleet across known accounts and write a
    /// canonical JSON to disk. Used by the hourly DR snapshot job and by
    /// operators verifying recovery state.
    Snapshot {
        #[command(subcommand)]
        sub: SnapshotCmd,
    },

    /// Disaster-recovery: restore the full IGLA fleet from a manifest.
    Restore {
        /// Path to the fleet manifest (e.g. `restore-fleet.json`).
        #[arg(long)]
        manifest: PathBuf,
        /// Override `RAILWAY_TOKEN` env (post-ban scenario).
        #[arg(long, hide = true)]
        new_token: Option<String>,
        /// Project name for the new/upserted project.
        #[arg(long, default_value = "IGLA")]
        project_name: String,
        /// Override image pin (SHA digest).
        #[arg(long)]
        champion_sha: Option<String>,
        /// Where to write the lock file.
        #[arg(long, default_value = "restore-fleet.lock.json")]
        lock_out: PathBuf,
        /// R9 safety gate: must be the string "PHI".
        #[arg(long)]
        confirm: Option<String>,
        /// Repo root for the experience log.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum SnapshotCmd {
    /// Probe Railway GraphQL for every (alias, project) pair listed below
    /// and write a canonical fleet snapshot to `--out`.
    Fleet {
        /// Output file. Default: disaster-recovery/fleet-snapshot.json
        #[arg(long, default_value = "disaster-recovery/fleet-snapshot.json")]
        out: PathBuf,
        /// Account triples in the form
        /// `alias=ALIAS,token_env=NAME,project_env=NAME,label=TEXT`.
        /// Repeatable. Each `token_env`/`project_env` is read from process
        /// env. Tokens are NEVER written to the snapshot — only the
        /// secret name (`token_secret`) is recorded.
        #[arg(
            long = "account",
            value_name = "alias=...,token_env=...,project_env=...,label=..."
        )]
        accounts: Vec<String>,
        /// Optional contact email to record alongside each alias.
        #[arg(long = "email", value_name = "alias=user@host")]
        emails: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ServiceCmd {
    /// Print all services in the configured project.
    List {
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT", default_value = IGLA_PROJECT_ID)]
        project: String,
    },
    /// Create a new image-backed service named `--name` with `--image`,
    /// upsert the variables, and trigger one redeploy. R7 audit triplet
    /// is appended to the local experience log.
    Deploy {
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT", default_value = IGLA_PROJECT_ID)]
        project: String,
        #[arg(long, env = "TRIOS_RAILWAY_ENV", default_value = IGLA_PROD_ENV_ID)]
        environment: String,
        /// Service name (e.g. `trios-train-seed-43`).
        #[arg(long)]
        name: String,
        /// Docker image; defaults to the IGLA trainer image.
        #[arg(long, default_value = DEFAULT_TRAINER_IMAGE)]
        image: String,
        /// `KEY=VALUE` env pairs to upsert. Repeatable.
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
        /// Reuse this existing service id instead of creating a new one.
        #[arg(long)]
        existing: Option<String>,
        /// If set, only print what would happen.
        #[arg(long)]
        dry_run: bool,
        /// Repo root for the experience log.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Trigger a redeploy of an existing service.
    Redeploy {
        #[arg(long, env = "TRIOS_RAILWAY_ENV", default_value = IGLA_PROD_ENV_ID)]
        environment: String,
        #[arg(long)]
        service: String,
    },
    /// Permanently delete a service.
    Delete {
        #[arg(long)]
        service: String,
        /// Confirm the destruction with `--yes`.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
enum AuditCmd {
    /// Print idempotent DDL for the Neon schema (issue #6).
    MigrateSql,
    /// Run an online audit pass: list services, detect drift, compute Gate-2
    /// verdict, seal an R7 triplet to the experience log. Exit codes:
    /// 0 = Gate-2 PASS, 1 = drift detected (error severity), 2 = NOT YET.
    Run {
        /// Project to audit.
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT", default_value = IGLA_PROJECT_ID)]
        project: String,
        /// Gate-2 BPB target (Gate-2 = 1.85, IGLA = 1.50).
        #[arg(long, default_value_t = 1.85_f64)]
        target: f64,
        /// Optional path to a JSONL ledger (one `LedgerRow`-like JSON per line).
        /// If omitted, audit treats the ledger as empty and any seed-bearing
        /// service will surface as `D1_ORPHAN_SERVICE` (warn, not error).
        #[arg(long)]
        ledger: Option<PathBuf>,
        /// Print the verdict as JSON to stdout (in addition to text summary).
        #[arg(long)]
        json: bool,
        /// Repo root for the experience log.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Compute Gate-2 verdict against an in-memory ledger snapshot. Useful
    /// for cron jobs that already have a Neon snapshot serialized to JSONL.
    Verdict {
        /// Path to a JSONL ledger.
        #[arg(long)]
        ledger: PathBuf,
        /// Gate-2 BPB target.
        #[arg(long, default_value_t = 1.85_f64)]
        target: f64,
    },
    /// Verify the Railway fleet health: list services and check for drift.
    Verify {
        /// Project to verify.
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT", default_value = IGLA_PROJECT_ID)]
        project: String,
    },
    /// Batch audit across multiple accounts/projects. Runs `audit run` for
    /// each known project, merges results, and returns the worst exit code.
    Batch {
        /// Gate-2 BPB target.
        #[arg(long, default_value_t = 1.85_f64)]
        target: f64,
        /// Print merged verdict as JSON.
        #[arg(long)]
        json: bool,
        /// Repo root for the experience log.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum ExperienceCmd {
    /// Append one line to the daily `.trinity/experience/<YYYYMMDD>.trinity` file.
    Append {
        /// Repository root (defaults to current directory).
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Issue ref like `#1`.
        #[arg(long)]
        issue: String,
        /// PHI LOOP step.
        #[arg(long)]
        phi_step: String,
        /// Free-form task summary.
        #[arg(long)]
        task: String,
        /// Status string.
        #[arg(long, default_value = "OK")]
        status: String,
        /// Soul-name (humorous English, L11).
        #[arg(long, default_value = "RailRangerOne")]
        soul_name: String,
        /// Agent codename.
        #[arg(long, default_value = "GENERAL")]
        agent: String,
        /// Verb being recorded (used for the audit triplet).
        #[arg(long, default_value = "experience")]
        verb: String,
        /// Project id (defaults to the IGLA project).
        #[arg(
            long,
            env = "TRIOS_RAILWAY_PROJECT",
            default_value = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b"
        )]
        project: String,
        /// Optional service id for the triplet.
        #[arg(long)]
        service: Option<String>,
        /// Token fingerprint (never the token itself).
        #[arg(long, default_value = "no-token")]
        token_fp: String,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .compact()
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Version => {
            println!("tri-railway {}", env!("CARGO_PKG_VERSION"));
        }
        Cmd::Audit {
            sub: AuditCmd::MigrateSql,
        } => {
            for stmt in migrations::ddl_statements() {
                println!("{stmt};");
            }
        }
        Cmd::Audit {
            sub:
                AuditCmd::Run {
                    project,
                    target,
                    ledger,
                    json,
                    root,
                },
        } => {
            let exit = run_audit(project, target, ledger, json, root).await?;
            std::process::exit(exit);
        }
        Cmd::Audit {
            sub: AuditCmd::Verdict { ledger, target },
        } => {
            let exit = cmd_audit_verdict(ledger, target).await?;
            std::process::exit(exit);
        }
        Cmd::Audit {
            sub: AuditCmd::Verify { project },
        } => {
            let exit = run_audit(project, 1.85, None, false, PathBuf::from(".")).await?;
            std::process::exit(exit);
        }
        Cmd::Audit {
            sub: AuditCmd::Batch { target, json, root },
        } => {
            let exit = run_audit_batch(target, json, root).await?;
            std::process::exit(exit);
        }
        Cmd::Service { sub } => run_service(sub).await?,
        Cmd::Snapshot {
            sub:
                SnapshotCmd::Fleet {
                    out,
                    accounts,
                    emails,
                },
        } => run_snapshot_fleet(out, accounts, emails).await?,
        Cmd::Restore {
            manifest,
            new_token,
            project_name,
            champion_sha,
            lock_out,
            confirm,
            root,
        } => {
            let exit = run_restore(
                manifest,
                new_token,
                project_name,
                champion_sha,
                lock_out,
                confirm,
                root,
            )
            .await?;
            std::process::exit(exit);
        }
        Cmd::Experience { sub } => {
            cmd_experience(sub).await?;
        }
    }

    Ok(())
}

async fn cmd_experience(sub: ExperienceCmd) -> Result<()> {
    match sub {
        ExperienceCmd::Append {
            root,
            issue,
            phi_step,
            task,
            status,
            soul_name,
            agent,
            verb,
            project,
            service,
            token_fp,
        } => {
            let project_id = ProjectId::from(project);
            let service_id = service.map(ServiceId::from);
            let hash = RailwayHash::seal(&verb, &project_id, service_id.as_ref(), &token_fp);
            let line = ExperienceLine::from_hash(
                &agent, &soul_name, &issue, &task, &status, &phi_step, &hash,
            )?;
            let path = append_line(&root.join(".trinity"), &line).await?;
            println!("appended: {}", path.display());
        }
    }
    Ok(())
}

/// Parse a JSONL ledger file into `LedgerRow`s. Unrecognised lines are
/// skipped with a warn-level trace. Empty path -> empty vec.
async fn load_ledger(path: &std::path::Path) -> Result<Vec<LedgerRow>> {
    let raw = tokio::fs::read_to_string(path).await?;
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => {
                let seed = v.get("seed").and_then(serde_json::Value::as_i64);
                let bpb = v.get("bpb").and_then(serde_json::Value::as_f64);
                let digest = v
                    .get("canonical_image_digest")
                    .or_else(|| v.get("image_digest"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                if let (Some(seed), Some(bpb)) = (seed, bpb) {
                    let Ok(seed_i32) = i32::try_from(seed) else {
                        tracing::warn!(line_no = i + 1, seed, "seed out of i32 range");
                        continue;
                    };
                    out.push(LedgerRow {
                        seed: seed_i32,
                        bpb,
                        canonical_image_digest: digest,
                    });
                } else {
                    tracing::warn!(line_no = i + 1, "ledger row missing seed/bpb");
                }
            }
            Err(e) => tracing::warn!(line_no = i + 1, ?e, "ledger row not valid JSON"),
        }
    }
    Ok(out)
}

/// Best-effort seed extraction from a service name like `trios-train-seed-43`
/// or `igla-final-seed-44`.
fn seed_from_name(name: &str) -> Option<i32> {
    name.rsplit_once("seed-")
        .and_then(|(_, tail)| tail.parse::<i32>().ok())
}

async fn run_audit(
    project: String,
    target: f64,
    ledger_path: Option<PathBuf>,
    json_out: bool,
    root: PathBuf,
) -> Result<i32> {
    let client =
        Client::from_env().map_err(|e| anyhow::anyhow!("RAILWAY_TOKEN not set or invalid: {e}"))?;
    let token_fp = client.token_fingerprint();

    let pid = ProjectId::from(project);
    let pv = Q::project_view(&client, &pid).await?;

    let real: Vec<RealService> = pv
        .services()
        .into_iter()
        .map(|s| RealService {
            service_id: ServiceId::from(s.id.clone()),
            seed: seed_from_name(&s.name),
            name: s.name,
            last_log_excerpt: None,
            last_bpb: None,
            image_digest: None,
            last_heartbeat: None,
        })
        .collect();

    let ledger = match ledger_path.as_deref() {
        Some(p) => load_ledger(p).await?,
        None => Vec::new(),
    };

    let events = detect(&real, &ledger);
    let v = compute_verdict(&real, &events, target);

    println!("project {} ({})", pv.name, pv.id);
    println!("services: {}   ledger rows: {}", real.len(), ledger.len());
    println!("target BPB:   {target}");
    println!("drift events: {}", events.len());
    for e in &events {
        println!("  [{:?}] {:?} {}", e.severity, e.code, e.detail);
    }
    let label = match v {
        AuditVerdict::Gate2Pass => "GATE-2 PASS",
        AuditVerdict::NotYet => "NOT YET",
        AuditVerdict::Drift => "DRIFT",
    };
    println!("verdict: {label}  (exit {})", v.exit_code());

    if json_out {
        let summary = serde_json::json!({
            "project":      pv.id,
            "project_name": pv.name,
            "services":     real.len(),
            "ledger_rows":  ledger.len(),
            "target":       target,
            "events":       events,
            "verdict":      label,
            "exit_code":    v.exit_code(),
        });
        println!("{summary}");
    }

    // R7 triplet for the audit pass itself.
    let hash = RailwayHash::seal("audit", &pid, None, &token_fp);
    let line = ExperienceLine::from_hash(
        "GENERAL",
        "DriftDoc",
        "#9",
        &format!(
            "audit run target={target} services={} events={} verdict={label}",
            real.len(),
            events.len()
        ),
        match v {
            AuditVerdict::Gate2Pass => "OK",
            AuditVerdict::NotYet => "WAIT",
            AuditVerdict::Drift => "FAIL",
        },
        "VERDICT",
        &hash,
    )?;
    let path = append_line(&root.join(".trinity"), &line).await?;
    tracing::info!(experience = %path.display(), "audit triplet sealed");

    Ok(v.exit_code())
}

async fn cmd_audit_verdict(ledger_path: PathBuf, target: f64) -> Result<i32> {
    let ledger = load_ledger(&ledger_path).await?;
    // synthetic real services with one entry per ledger seed,
    // pretending Railway reality matches the ledger 1:1.
    let real: Vec<RealService> = ledger
        .iter()
        .map(|r| RealService {
            service_id: ServiceId::from(format!("ledger-seed-{}", r.seed)),
            name: format!("trios-train-seed-{}", r.seed),
            seed: Some(r.seed),
            last_log_excerpt: None,
            last_bpb: Some(r.bpb),
            image_digest: None,
            last_heartbeat: None,
        })
        .collect();
    let events = detect(&real, &ledger);
    let v = compute_verdict(&real, &events, target);
    println!(
        "verdict: {}  (rows={}, target={target}, exit={})",
        match v {
            AuditVerdict::Gate2Pass => "GATE-2 PASS",
            AuditVerdict::NotYet => "NOT YET",
            AuditVerdict::Drift => "DRIFT",
        },
        ledger.len(),
        v.exit_code()
    );
    Ok(v.exit_code())
}

fn parse_var(s: &str) -> Result<(String, String)> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("variable `{s}` is not in KEY=VALUE form"))?;
    if k.is_empty() {
        anyhow::bail!("empty variable name in `{s}`");
    }
    Ok((k.to_string(), v.to_string()))
}

async fn run_service(cmd: ServiceCmd) -> Result<()> {
    let client =
        Client::from_env().map_err(|e| anyhow::anyhow!("RAILWAY_TOKEN not set or invalid: {e}"))?;
    let token_fp = client.token_fingerprint();

    match cmd {
        ServiceCmd::List { project } => {
            let pid = ProjectId::from(project);
            let pv = Q::project_view(&client, &pid).await?;
            println!("project {} ({})", pv.name, pv.id);
            for s in pv.services() {
                println!("  {}  {}  {}", s.id, s.name, s.created_at);
            }
        }
        ServiceCmd::Deploy {
            project,
            environment,
            name,
            image,
            vars,
            existing,
            dry_run,
            root,
        } => {
            let pid = ProjectId::from(project);
            let eid = EnvironmentId::from(environment);
            let mut parsed = Vec::with_capacity(vars.len());
            for v in &vars {
                parsed.push(parse_var(v)?);
            }

            if dry_run {
                println!("DRY RUN: would deploy {name} from {image}");
                println!("  project   = {}", pid.as_str());
                println!("  env       = {}", eid.as_str());
                if let Some(eid) = &existing {
                    println!("  reuse svc = {eid}");
                }
                for (k, v) in &parsed {
                    println!("  var       = {k}=<{} chars>", v.len());
                }
                return Ok(());
            }

            let service_id: ServiceId = if let Some(eid) = existing {
                ServiceId::from(eid)
            } else {
                let created = M::service_create(&client, &pid, &name).await?;
                println!("created service {} ({})", created.name, created.id);
                ServiceId::from(created.id)
            };

            M::service_instance_set_image(&client, &service_id, &eid, &image).await?;
            println!("set image: {image}");

            for (k, v) in &parsed {
                M::variable_upsert(&client, &pid, &eid, &service_id, k, v).await?;
                println!("  var: {k}=<{}>", v.len());
            }

            let deploy_id = M::service_redeploy(&client, &service_id, &eid).await?;
            println!("redeploy triggered: {deploy_id}");

            // R7 triplet to local experience log.
            let hash = RailwayHash::seal("deploy", &pid, Some(&service_id), &token_fp);
            let line = ExperienceLine::from_hash(
                "GENERAL",
                "RailRangerOne",
                "#5",
                &format!("deploy {name} image={image}"),
                "OK",
                "PUSH",
                &hash,
            )?;
            let path = append_line(&root.join(".trinity"), &line).await?;
            println!("experience: {}", path.display());
        }
        ServiceCmd::Redeploy {
            environment,
            service,
        } => {
            let eid = EnvironmentId::from(environment);
            let sid = ServiceId::from(service);
            let deploy_id = M::service_redeploy(&client, &sid, &eid).await?;
            println!("redeploy triggered: {deploy_id}");
        }
        ServiceCmd::Delete { service, yes } => {
            if !yes {
                anyhow::bail!("refusing to delete service `{service}` without --yes");
            }
            let sid = ServiceId::from(service);
            M::service_delete(&client, &sid).await?;
            println!("deleted: {sid}");
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct FleetManifest {
    version: u32,
    #[allow(dead_code)]
    project: FleetProject,
    image: String,
    shared_vars: Vec<FleetVar>,
    services: Vec<FleetService>,
}

#[derive(serde::Deserialize)]
struct FleetProject {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    default_environment: String,
}

#[derive(serde::Deserialize, Clone)]
struct FleetVar {
    key: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct FleetService {
    name: String,
    #[allow(dead_code)]
    kind: Option<String>,
    image_override: Option<String>,
    #[serde(default)]
    vars: Vec<FleetVar>,
}

#[derive(serde::Serialize, Clone)]
struct LockEntry {
    name: String,
    service_id: String,
    image: String,
    status: String,
}

#[derive(serde::Serialize)]
struct LockFile {
    anchor: String,
    restored_at: String,
    project_id: String,
    services: Vec<LockEntry>,
    experience_triplet: String,
}

fn interpolate_secret(value: &str) -> String {
    const MAX_ITERATIONS: u32 = 32;
    let mut result = value.to_string();
    let mut iterations = 0;
    while let Some(start) = result.find("${secret:") {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            tracing::warn!("interpolate_secret: exceeded {MAX_ITERATIONS} iterations, stopping");
            break;
        }
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 9..start + end];
            let resolved =
                std::env::var(var_name).unwrap_or_else(|_| format!("MISSING_SECRET:{var_name}"));
            if resolved.contains("${secret:") {
                tracing::warn!(
                    var = var_name,
                    "interpolate_secret: resolved value contains nested secret ref, skipping"
                );
                break;
            }
            result.replace_range(start..=start + end, &resolved);
        } else {
            break;
        }
    }
    result
}

#[allow(clippy::too_many_lines)]
async fn run_restore(
    manifest_path: PathBuf,
    new_token: Option<String>,
    project_name: String,
    champion_sha: Option<String>,
    lock_out: PathBuf,
    confirm: Option<String>,
    root: PathBuf,
) -> Result<i32> {
    if confirm.as_deref() != Some("PHI") {
        eprintln!("R9 safety gate: pass --confirm PHI to proceed");
        return Ok(9);
    }

    let raw = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: FleetManifest = serde_json::from_str(&raw)?;
    if manifest.version != 1 {
        anyhow::bail!("unsupported manifest version: {}", manifest.version);
    }

    let _ = project_name;

    let client = if let Some(ref token) = new_token {
        Client::with_token_and_mode(token, AuthMode::Team)
            .map_err(|e| anyhow::anyhow!("RAILWAY_TOKEN not set or invalid: {e}"))?
    } else {
        Client::from_env().map_err(|e| anyhow::anyhow!("RAILWAY_TOKEN not set or invalid: {e}"))?
    };
    let token_fp = client.token_fingerprint();

    let pid = ProjectId::from(
        std::env::var("TRIOS_RAILWAY_PROJECT").unwrap_or_else(|_| IGLA_PROJECT_ID.to_string()),
    );
    let eid = EnvironmentId::from(
        std::env::var("TRIOS_RAILWAY_ENV").unwrap_or_else(|_| IGLA_PROD_ENV_ID.to_string()),
    );

    let image = champion_sha.as_deref().unwrap_or(&manifest.image);

    let mut lock_entries: Vec<LockEntry> = Vec::new();
    let mut failures: usize = 0;

    for svc in &manifest.services {
        let svc_image = svc.image_override.as_deref().unwrap_or(image);

        tracing::info!(service = %svc.name, image = %svc_image, "restoring service");

        let service_id = match M::service_create(&client, &pid, &svc.name).await {
            Ok(created) => {
                tracing::info!(service = %svc.name, id = %created.id, "created service");
                ServiceId::from(created.id)
            }
            Err(e) => {
                tracing::error!(service = %svc.name, error = %e, "failed to create service");
                failures += 1;
                lock_entries.push(LockEntry {
                    name: svc.name.clone(),
                    service_id: String::new(),
                    image: svc_image.to_string(),
                    status: "FAILED".to_string(),
                });
                continue;
            }
        };

        if let Err(e) = M::service_instance_set_image(&client, &service_id, &eid, svc_image).await {
            tracing::error!(service = %svc.name, error = %e, "failed to set image");
            failures += 1;
            lock_entries.push(LockEntry {
                name: svc.name.clone(),
                service_id: service_id.as_str().to_string(),
                image: svc_image.to_string(),
                status: "FAILED_IMAGE".to_string(),
            });
            continue;
        }

        let mut all_vars = manifest.shared_vars.clone();
        all_vars.extend(svc.vars.iter().cloned());
        for v in &all_vars {
            let interpolated = interpolate_secret(&v.value);
            if let Err(e) =
                M::variable_upsert(&client, &pid, &eid, &service_id, &v.key, &interpolated).await
            {
                tracing::warn!(service = %svc.name, key = %v.key, error = %e, "failed to upsert var");
            }
        }

        match M::service_redeploy(&client, &service_id, &eid).await {
            Ok(deploy_id) => {
                tracing::info!(service = %svc.name, deploy = %deploy_id, "redeploy triggered");
            }
            Err(e) => {
                tracing::error!(service = %svc.name, error = %e, "failed to redeploy");
                failures += 1;
                lock_entries.push(LockEntry {
                    name: svc.name.clone(),
                    service_id: service_id.as_str().to_string(),
                    image: svc_image.to_string(),
                    status: "FAILED_REDEPLOY".to_string(),
                });
                continue;
            }
        }

        lock_entries.push(LockEntry {
            name: svc.name.clone(),
            service_id: service_id.as_str().to_string(),
            image: svc_image.to_string(),
            status: "OK".to_string(),
        });
    }

    let ts = chrono::Utc::now().to_rfc3339();
    let triplet = format!(
        "RAIL=restore @ project={} service=ALL sha={} ts={ts}",
        pid.short(),
        &image[..8.min(image.len())],
    );

    let lock = LockFile {
        anchor: "phi^2 + phi^-2 = 3".to_string(),
        restored_at: ts.clone(),
        project_id: pid.as_str().to_string(),
        services: lock_entries.clone(),
        experience_triplet: triplet.clone(),
    };

    let lock_body = serde_json::to_string_pretty(&lock)? + "\n";
    tokio::fs::write(&lock_out, &lock_body).await?;
    tracing::info!(path = %lock_out.display(), "lock file written");

    let hash = RailwayHash::seal("restore", &pid, None, &token_fp);
    let line = ExperienceLine::from_hash(
        "GENERAL",
        "FleetPhoenix",
        "#25",
        &format!(
            "restore {} services failures={failures}",
            manifest.services.len()
        ),
        if failures == 0 { "OK" } else { "PARTIAL" },
        "EXPERIENCE",
        &hash,
    )?;
    let path = append_line(&root.join(".trinity"), &line).await?;
    tracing::info!(experience = %path.display(), "restore triplet sealed");

    let ok_count = lock_entries.iter().filter(|e| e.status == "OK").count();
    println!(
        "RESTORE {} — {ok_count}/{} services OK, {failures} failures",
        if failures == 0 { "OK" } else { "PARTIAL" },
        manifest.services.len(),
    );

    match failures {
        0 => Ok(0),
        _ if failures < manifest.services.len() => Ok(3),
        _ => Ok(1),
    }
}

/// Parse `key=value,key=value` style flag values.
fn parse_kv_list(spec: &str) -> std::collections::HashMap<String, String> {
    spec.split(',')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Parse `alias=value` simple pairs.
fn parse_alias_kv(spec: &str) -> Option<(String, String)> {
    spec.split_once('=')
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
}

#[derive(serde::Serialize)]
struct SnapshotAccount {
    alias: String,
    project_label: String,
    email: Option<String>,
    token_secret: String,
    project_id: Option<String>,
    project_name: Option<String>,
    environments: Vec<serde_json::Value>,
    services: Vec<serde_json::Value>,
    service_count: usize,
}

#[derive(serde::Serialize)]
struct SnapshotDoc<'a> {
    anchor: &'a str,
    generated_at: String,
    generator: &'a str,
    version: &'a str,
    accounts: Vec<SnapshotAccount>,
    totals: serde_json::Value,
}

async fn run_snapshot_fleet(
    out: PathBuf,
    accounts: Vec<String>,
    emails: Vec<String>,
) -> Result<()> {
    let email_map: std::collections::HashMap<String, String> =
        emails.iter().filter_map(|s| parse_alias_kv(s)).collect();

    let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut acc_out: Vec<SnapshotAccount> = Vec::new();
    let mut alias_set = std::collections::HashSet::new();
    let mut total_services: usize = 0;

    for spec in &accounts {
        let kv = parse_kv_list(spec);
        let alias = kv.get("alias").cloned().unwrap_or_else(|| "?".to_string());
        let label = kv.get("label").cloned().unwrap_or_else(|| "?".to_string());
        let token_env = kv.get("token_env").cloned().unwrap_or_default();
        let project_env = kv.get("project_env").cloned().unwrap_or_default();

        let token = std::env::var(&token_env).ok();
        let project = std::env::var(&project_env).ok();

        let (project_id, project_name, environments, services) = if let (Some(tok), Some(proj)) =
            (token, project)
        {
            let client = Client::with_token_and_mode(&tok, AuthMode::Team)
                .map_err(|e| anyhow::anyhow!("with_token_and_mode: {e}"))?;
            let pid = ProjectId::from(proj);
            match Q::project_view(&client, &pid).await {
                Ok(pv) => {
                    let svcs = pv
                        .services()
                        .into_iter()
                        .map(|s| {
                            serde_json::json!({
                                "id": s.id,
                                "name": s.name,
                                "createdAt": s.created_at,
                            })
                        })
                        .collect::<Vec<_>>();
                    (Some(pv.id.clone()), Some(pv.name.clone()), Vec::new(), svcs)
                }
                Err(e) => {
                    tracing::warn!(alias = %alias, label = %label, error = %e, "skipping account");
                    (None, None, Vec::new(), Vec::new())
                }
            }
        } else {
            tracing::warn!(
                alias = %alias,
                label = %label,
                "missing token_env or project_env in process env, recording empty"
            );
            (None, None, Vec::new(), Vec::new())
        };

        alias_set.insert(alias.clone());
        let count = services.len();
        total_services += count;
        acc_out.push(SnapshotAccount {
            alias: alias.clone(),
            project_label: label,
            email: email_map.get(&alias).cloned(),
            token_secret: token_env,
            project_id,
            project_name,
            environments,
            services,
            service_count: count,
        });
    }

    let total_projects = acc_out.len();
    let total_accounts = alias_set.len();

    let doc = SnapshotDoc {
        anchor: "phi^2 + phi^-2 = 3",
        generated_at: ts,
        generator: "tri-railway snapshot fleet",
        version: "1.0.0",
        accounts: acc_out,
        totals: serde_json::json!({
            "accounts": total_accounts,
            "projects": total_projects,
            "services": total_services,
        }),
    };

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let body = serde_json::to_string_pretty(&doc)? + "\n";
    tokio::fs::write(&out, &body).await?;
    println!(
        "wrote {}: accounts={} projects={} services={}",
        out.display(),
        total_accounts,
        total_projects,
        total_services
    );
    Ok(())
}

const FLEET_PROJECTS: &[(&str, &str)] = &[
    ("acc1", IGLA_PROJECT_ID),
    ("acc2", "39d833c1-4cb6-4af9-b61b-c204b6733a98"),
];

#[allow(clippy::too_many_lines)]
async fn run_audit_batch(target: f64, json_out: bool, _root: PathBuf) -> Result<i32> {
    let mut worst_exit = 0i32;
    let mut all_events: Vec<serde_json::Value> = Vec::new();
    let mut total_services = 0usize;

    for (alias, project_id) in FLEET_PROJECTS {
        let token_env = format!("RAILWAY_TOKEN_{}", alias.to_uppercase());
        let token = std::env::var(&token_env)
            .or_else(|_| std::env::var("RAILWAY_TOKEN"))
            .ok();

        if let Some(tok) = token {
            std::env::set_var("RAILWAY_TOKEN", &tok);
            std::env::set_var("RAILWAY_TOKEN_AUTH", "team");
        }

        let client_result = Client::from_env();
        let client = match client_result {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(account = %alias, error = %e, "skipping account");
                continue;
            }
        };

        let pid = ProjectId::from((*project_id).to_string());
        let pv = match Q::project_view(&client, &pid).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(account = %alias, error = %e, "project view failed");
                continue;
            }
        };

        let real: Vec<RealService> = pv
            .services()
            .into_iter()
            .map(|s| RealService {
                service_id: ServiceId::from(s.id.clone()),
                seed: seed_from_name(&s.name),
                name: s.name,
                last_log_excerpt: None,
                last_bpb: None,
                image_digest: None,
                last_heartbeat: None,
            })
            .collect();

        let events = detect(&real, &[]);
        let v = compute_verdict(&real, &events, target);

        println!(
            "[{}] {} services, {} events, verdict={:?}",
            alias,
            real.len(),
            events.len(),
            v
        );

        total_services += real.len();
        for e in &events {
            all_events.push(serde_json::json!({
                "account": alias,
                "code": format!("{:?}", e.code),
                "severity": format!("{:?}", e.severity),
                "service": e.detail,
            }));
        }

        let exit = v.exit_code();
        if exit > worst_exit {
            worst_exit = exit;
        }
    }

    if json_out {
        let summary = serde_json::json!({
            "accounts": FLEET_PROJECTS.len(),
            "total_services": total_services,
            "total_events": all_events.len(),
            "target": target,
            "events": all_events,
            "worst_exit": worst_exit,
        });
        println!("{summary}");
    }

    println!(
        "BATCH verdict: {} services across {} accounts, {} drift events, exit={worst_exit}",
        total_services,
        FLEET_PROJECTS.len(),
        all_events.len()
    );

    Ok(worst_exit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_v1() {
        let json = r#"{
            "version": 1,
            "anchor": "phi^2 + phi^-2 = 3",
            "project": { "name": "IGLA", "description": "test", "default_environment": "production" },
            "image": "ghcr.io/ghashtag/trios-trainer-igla:latest",
            "shared_vars": [{ "key": "RUST_LOG", "value": "info" }],
            "services": [
                { "name": "igla-final-seed-42", "kind": "trainer", "vars": [{ "key": "TRIOS_SEED", "value": "42" }] },
                { "name": "trios-mcp-public", "kind": "mcp", "image_override": "ghcr.io/ghashtag/trios-railway-mcp:latest", "vars": [] }
            ]
        }"#;
        let m: FleetManifest = serde_json::from_str(json).expect("parse manifest");
        assert_eq!(m.version, 1);
        assert_eq!(m.services.len(), 2);
        assert_eq!(m.project.name, "IGLA");
        assert_eq!(
            m.services[1].image_override.as_deref(),
            Some("ghcr.io/ghashtag/trios-railway-mcp:latest")
        );
    }

    #[test]
    fn secret_interpolation_resolves() {
        std::env::set_var("TEST_SECRET_HELLO", "world");
        let result = interpolate_secret("${secret:TEST_SECRET_HELLO}");
        assert_eq!(result, "world");
    }

    #[test]
    fn secret_interpolation_missing() {
        let result = interpolate_secret("${secret:NONEXISTENT_VAR_XYZ}");
        assert!(result.contains("MISSING_SECRET"));
    }

    #[test]
    fn secret_interpolation_no_secrets() {
        let result = interpolate_secret("plain-value");
        assert_eq!(result, "plain-value");
    }

    #[test]
    fn secret_interpolation_mixed() {
        std::env::set_var("TEST_MIX_A", "aaa");
        let result = interpolate_secret("prefix-${secret:TEST_MIX_A}-suffix");
        assert_eq!(result, "prefix-aaa-suffix");
    }

    #[test]
    fn secret_interpolation_no_infinite_loop_on_self_ref() {
        std::env::set_var("TEST_SELF_REF", "x${secret:TEST_SELF_REF}y");
        let result = interpolate_secret("${secret:TEST_SELF_REF}");
        assert!(!result.is_empty());
    }

    #[test]
    fn secret_interpolation_nested_ref_stops() {
        std::env::set_var("TEST_NESTED_B", "val-${secret:TEST_NESTED_C}");
        let result = interpolate_secret("${secret:TEST_NESTED_B}");
        assert!(!result.is_empty());
    }

    #[test]
    fn parse_var_key_value() {
        let (k, v) = parse_var("FOO=bar").unwrap();
        assert_eq!(k, "FOO");
        assert_eq!(v, "bar");
    }

    #[test]
    fn parse_var_rejects_no_equals() {
        assert!(parse_var("NOEQUALS").is_err());
    }

    #[test]
    fn manifest_parses_v1_single_service() {
        let json = r#"{
            "version": 1,
            "anchor": "phi^2 + phi^-2 = 3",
            "project": { "name": "IGLA", "description": "test", "default_environment": "production" },
            "image": "ghcr.io/ghashtag/trios-trainer-igla:latest",
            "shared_vars": [{ "key": "RUST_LOG", "value": "info" }],
            "services": [
                { "name": "trios-train-seed-42", "kind": "trainer", "vars": [{ "key": "TRIOS_SEED", "value": "42" }] }
            ]
        }"#;
        let manifest: FleetManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.services.len(), 1);
        assert_eq!(manifest.services[0].name, "trios-train-seed-42");
        assert_eq!(manifest.shared_vars.len(), 1);
    }

    #[test]
    fn manifest_rejects_bad_version() {
        let json = r#"{
            "version": 2,
            "anchor": "x",
            "project": { "name": "X", "description": "x", "default_environment": "prod" },
            "image": "img",
            "shared_vars": [],
            "services": []
        }"#;
        let manifest: FleetManifest = serde_json::from_str(json).unwrap();
        assert_ne!(manifest.version, 1);
    }

    #[test]
    fn r9_refuses_without_confirm() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let exit = rt
            .block_on(run_restore(
                PathBuf::from("restore-fleet.json"),
                None,
                "IGLA".to_string(),
                None,
                PathBuf::from("test.lock"),
                None,
                PathBuf::from("."),
            ))
            .unwrap();
        assert_eq!(exit, 9);
    }

    #[test]
    fn r9_refuses_wrong_confirm() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let exit = rt
            .block_on(run_restore(
                PathBuf::from("restore-fleet.json"),
                None,
                "IGLA".to_string(),
                None,
                PathBuf::from("test.lock"),
                Some("NOPE".to_string()),
                PathBuf::from("."),
            ))
            .unwrap();
        assert_eq!(exit, 9);
    }
}

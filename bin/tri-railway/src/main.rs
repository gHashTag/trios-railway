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
    is_uuid_like, mutations as M, queries as Q, AuthMode, Client, EnvironmentId, ProjectId,
    RailwayHash, ServiceId,
};
use trios_railway_experience::{append_line, ExperienceLine};

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
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT")]
        project: String,
    },
    /// Create a new image-backed service named `--name` with `--image`,
    /// upsert the variables, and trigger one redeploy. R7 audit triplet
    /// is appended to the local experience log.
    Deploy {
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT")]
        project: String,
        #[arg(long, env = "TRIOS_RAILWAY_ENV")]
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
    /// Deploy multiple experiments from TOML configs in parallel across
    /// all available accounts. Reads each `.toml`, converts to Railway
    /// service vars, and deploys to the account with the most free slots.
    BatchDeploy {
        /// Path to a single TOML experiment file or a directory of TOML files.
        #[arg(long, default_value = "experiments")]
        source: PathBuf,
        /// Docker image; defaults to the IGLA trainer image.
        #[arg(long, default_value = DEFAULT_TRAINER_IMAGE)]
        image: String,
        /// Account index (0-3) to force; auto-selects if omitted.
        #[arg(long)]
        account: Option<usize>,
        /// Maximum concurrent deploys.
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
        /// If set, only print what would happen.
        #[arg(long)]
        dry_run: bool,
        /// Repo root for the experience log.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Trigger a redeploy of an existing service.
    Redeploy {
        #[arg(long, env = "TRIOS_RAILWAY_ENV")]
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
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT")]
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
        #[arg(long, env = "TRIOS_RAILWAY_PROJECT")]
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
        Cmd::Service { sub } => run_service(sub).await?,
        Cmd::Snapshot {
            sub:
                SnapshotCmd::Fleet {
                    out,
                    accounts,
                    emails,
                },
        } => run_snapshot_fleet(out, accounts, emails).await?,
        Cmd::Experience { sub } => match sub {
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
        },
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

// ── TOML experiment config ──────────────────────────────────────────

/// Minimal TOML experiment config matching the `experiments/*.toml` schema.
#[derive(Debug, Clone, serde::Deserialize)]
struct ExperimentToml {
    experiment: ExperimentMeta,
    #[serde(default)]
    model: Option<TomlModel>,
    #[serde(default)]
    training: Option<TomlTraining>,
    #[serde(default)]
    quantization: Option<TomlQuantization>,
    #[serde(default)]
    meta: Option<TomlMeta>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ExperimentMeta {
    name: String,
    seed: u64,
    #[serde(default)]
    priority: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TomlModel {
    #[serde(default)]
    hidden_dim: Option<u64>,
    #[serde(default)]
    context_len: Option<u64>,
    #[serde(default)]
    architecture: Option<String>,
    #[serde(default)]
    attention: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TomlTraining {
    #[serde(default)]
    steps_budget: Option<u64>,
    #[serde(default)]
    warmup_steps: Option<u64>,
    #[serde(default)]
    learning_rate: Option<f64>,
    #[serde(default)]
    batch_size: Option<u64>,
    #[serde(default)]
    gradient_accumulation_steps: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TomlQuantization {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TomlMeta {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    target_bpb: Option<f64>,
}

impl ExperimentToml {
    /// Convert the TOML experiment into Railway environment variables.
    fn to_service_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();

        vars.push(("SEED".into(), self.experiment.seed.to_string()));
        vars.push(("EXPERIMENT_NAME".into(), self.experiment.name.clone()));

        if let Some(m) = &self.model {
            if let Some(v) = m.hidden_dim {
                vars.push(("HIDDEN_DIM".into(), v.to_string()));
            }
            if let Some(v) = m.context_len {
                vars.push(("CONTEXT_LEN".into(), v.to_string()));
            }
            if let Some(v) = &m.architecture {
                vars.push(("ARCHITECTURE".into(), v.clone()));
            }
            if let Some(v) = m.attention {
                vars.push(("ATTENTION".into(), v.to_string()));
            }
        }

        if let Some(t) = &self.training {
            if let Some(v) = t.steps_budget {
                vars.push(("STEPS_BUDGET".into(), v.to_string()));
            }
            if let Some(v) = t.warmup_steps {
                vars.push(("WARMUP_STEPS".into(), v.to_string()));
            }
            if let Some(v) = t.learning_rate {
                vars.push(("LEARNING_RATE".into(), v.to_string()));
            }
            if let Some(v) = t.batch_size {
                vars.push(("BATCH_SIZE".into(), v.to_string()));
            }
            if let Some(v) = t.gradient_accumulation_steps {
                vars.push(("GRADIENT_ACCUMULATION_STEPS".into(), v.to_string()));
            }
        }

        if let Some(q) = &self.quantization {
            if let Some(v) = q.enabled {
                vars.push(("QUANTIZATION_ENABLED".into(), v.to_string()));
            }
            if let Some(v) = &q.format {
                vars.push(("QUANTIZATION_FORMAT".into(), v.clone()));
            }
        }

        if let Some(m) = &self.meta {
            if let Some(v) = &m.category {
                vars.push(("CATEGORY".into(), v.clone()));
            }
            if let Some(v) = m.target_bpb {
                vars.push(("TARGET_BPB".into(), v.to_string()));
            }
        }

        vars
    }

    /// Derive a Railway service name from the experiment config.
    fn service_name(&self) -> String {
        let name = &self.experiment.name;
        let seed = self.experiment.seed;
        // Sanitise: replace non-alphanumeric with hyphens, lowercase.
        let clean: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        format!("trios-train-seed-{seed}-{clean}")
    }
}

// ── Account management ──────────────────────────────────────────────

/// A Railway account with its token, project, and environment.
struct AccountSlot {
    index: usize,
    token: String,
    project_id: String,
    environment_id: String,
    token_kind: String,
    service_count: usize,
}

impl AccountSlot {
    /// Read all accounts from environment variables (ACC0..ACC3).
    fn from_env() -> Vec<Self> {
        let mut accounts = Vec::new();
        for i in 0..=3_u8 {
            let token_key = format!("RAILWAY_TOKEN_ACC{i}");
            let project_key = format!("RAILWAY_PROJECT_ID_ACC{i}");
            let env_key = format!("RAILWAY_ENVIRONMENT_ID_ACC{i}");
            let kind_key = format!("RAILWAY_TOKEN_KIND_ACC{i}");

            let Ok(token) = std::env::var(&token_key) else {
                continue;
            };
            let project_id = std::env::var(&project_key).unwrap_or_default();
            let environment_id = std::env::var(&env_key).unwrap_or_default();
            let token_kind = std::env::var(&kind_key).unwrap_or_else(|_| "team".into());

            accounts.push(Self {
                index: usize::from(i),
                token,
                project_id,
                environment_id,
                token_kind,
                service_count: 0,
            });
        }
        accounts
    }

    /// Build a `Client` for this account, setting the correct auth mode.
    fn client(&self) -> Result<Client> {
        let auth = match self.token_kind.as_str() {
            "project" => trios_railway_core::AuthMode::Project,
            _ => trios_railway_core::AuthMode::Team,
        };
        Client::with_token_and_mode(&self.token, auth)
            .map_err(|e| anyhow::anyhow!("client for ACC{}: {e}", self.index))
    }
}

/// Pick the account with the fewest services (most free slots).
fn pick_account(accounts: &[AccountSlot], force: Option<usize>) -> Option<&AccountSlot> {
    if let Some(idx) = force {
        return accounts.iter().find(|a| a.index == idx);
    }
    accounts.iter().min_by_key(|a| a.service_count)
}

/// Collect TOML files from a path (single file or directory).
fn collect_toml_files(source: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    if source.is_file() {
        return Ok(vec![source.to_path_buf()]);
    }
    if source.is_dir() {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                files.push(path);
            }
        }
        files.sort();
        return Ok(files);
    }
    anyhow::bail!("source path does not exist: {}", source.display());
}

// ── Service commands ────────────────────────────────────────────────

async fn run_service(cmd: ServiceCmd) -> Result<()> {
    if let ServiceCmd::BatchDeploy {
        source,
        image,
        account,
        concurrency,
        dry_run,
        root,
    } = cmd
    {
        return run_batch_deploy(source, image, account, concurrency, dry_run, root).await;
    }

    // Single-service commands need a single client from RAILWAY_TOKEN.
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
            return cmd_deploy(
                &client, &token_fp, project, environment, name, image, vars, existing, dry_run, root,
            )
            .await;
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
        ServiceCmd::BatchDeploy { .. } => unreachable!(),
    }
    Ok(())
}

/// Handle the `service deploy` subcommand.
#[allow(clippy::too_many_arguments)]
async fn cmd_deploy(
    client: &Client,
    token_fp: &str,
    project: String,
    environment: String,
    name: String,
    image: String,
    vars: Vec<String>,
    existing: Option<String>,
    dry_run: bool,
    root: PathBuf,
) -> Result<()> {
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
            println!("  var       = {k}={v}");
        }
        return Ok(());
    }

    let service_id: ServiceId = if let Some(eid) = existing {
        ServiceId::from(eid)
    } else {
        let created = M::service_create(client, &pid, &name).await?;
        println!("created service {} ({})", created.name, created.id);
        ServiceId::from(created.id)
    };

    M::service_instance_set_image(client, &service_id, &eid, &image).await?;
    println!("set image: {image}");

    // Parallel variable upsert for speed.
    let ok = M::variables_upsert_parallel(client, &pid, &eid, &service_id, &parsed).await?;
    println!("  vars: {ok}/{} upserted", parsed.len());

    let deploy_id = M::service_redeploy(client, &service_id, &eid).await?;
    println!("redeploy triggered: {deploy_id}");

    // R7 triplet to local experience log.
    let hash = RailwayHash::seal("deploy", &pid, Some(&service_id), token_fp);
    let line = ExperienceLine::from_hash(
        "GENERAL",
        "RailRangerOne",
        "#81",
        &format!("deploy {name} image={image}"),
        "OK",
        "PUSH",
        &hash,
    )?;
    let path = append_line(&root.join(".trinity"), &line).await?;
    println!("experience: {}", path.display());
    Ok(())
}

/// Batch-deploy experiments from TOML files across multiple accounts.
async fn run_batch_deploy(
    source: std::path::PathBuf,
    image: String,
    account: Option<usize>,
    concurrency: usize,
    dry_run: bool,
    root: PathBuf,
) -> Result<()> {
    let toml_files = collect_toml_files(&source)?;
    if toml_files.is_empty() {
        anyhow::bail!("no .toml files found in {}", source.display());
    }

    println!(
        "batch-deploy: {} experiments, concurrency={concurrency}",
        toml_files.len()
    );

    // Parse all TOML files.
    let mut experiments: Vec<(std::path::PathBuf, ExperimentToml)> = Vec::new();
    for path in &toml_files {
        let content = std::fs::read_to_string(path)?;
        let exp: ExperimentToml = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("parse {}: {e}", path.display()))?;
        experiments.push((path.clone(), exp));
    }

    // Sort by priority (descending).
    experiments.sort_by_key(|b| std::cmp::Reverse(b.1.experiment.priority));

    // Load accounts and count services.
    let mut accounts = AccountSlot::from_env();
    if accounts.is_empty() {
        anyhow::bail!("no RAILWAY_TOKEN_ACC* found in environment");
    }

    for acc in &mut accounts {
        match acc.client() {
            Ok(client) => {
                let pid = ProjectId::from(acc.project_id.clone());
                match Q::project_view(&client, &pid).await {
                    Ok(pv) => acc.service_count = pv.services().len(),
                    Err(e) => {
                        tracing::warn!(acc = acc.index, error = %e, "cannot query project");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(acc = acc.index, error = %e, "cannot build client");
            }
        }
    }

    println!("accounts:");
    for acc in &accounts {
        println!(
            "  ACC{}: {} services ({})",
            acc.index, acc.service_count, acc.project_id
        );
    }

    if dry_run {
        println!("\nDRY RUN — would deploy:");
        for (path, exp) in &experiments {
            let svc_name = exp.service_name();
            let vars = exp.to_service_vars();
            let chosen = pick_account(&accounts, account);
            let acc_label = chosen.map_or_else(|| "NONE".into(), |a| format!("ACC{}", a.index));
            println!("  {acc_label}: {svc_name} ({} vars) [{}]", vars.len(), path.display());
        }
        return Ok(());
    }

    // Deploy with bounded concurrency.
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();

    for (_path, exp) in experiments {
        let sem = sem.clone();
        let image = image.clone();
        let root = root.clone();
        let accounts_snapshot = accounts.clone();

        let handle = tokio::spawn(deploy_one_experiment(
            sem, accounts_snapshot, account, exp, image, root,
        ));
        handles.push(handle);
    }

    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => ok_count += 1,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "batch-deploy task failed");
                err_count += 1;
            }
            Err(e) => {
                tracing::error!(error = %e, "batch-deploy task panicked");
                err_count += 1;
            }
        }
    }

    println!(
        "\nbatch-deploy complete: {ok_count} ok, {err_count} failed, {} total",
        ok_count + err_count
    );

    if err_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Deploy a single experiment to the best available account.
/// Called inside a `tokio::spawn` with a semaphore permit.
async fn deploy_one_experiment(
    sem: std::sync::Arc<tokio::sync::Semaphore>,
    accounts: Vec<AccountSlot>,
    account: Option<usize>,
    exp: ExperimentToml,
    image: String,
    root: PathBuf,
) -> Result<()> {
    let _permit = sem.acquire().await.unwrap();

    let acc = pick_account(&accounts, account);
    let Some(acc) = acc else {
        tracing::error!(exp = %exp.experiment.name, "no account available");
        return Err(anyhow::anyhow!("no account available"));
    };

    let client = acc.client()?;
    let pid = ProjectId::from(acc.project_id.clone());
    let eid = EnvironmentId::from(acc.environment_id.clone());
    let svc_name = exp.service_name();
    let vars = exp.to_service_vars();

    println!(
        "  [ACC{}] deploying {svc_name} ({} vars)...",
        acc.index,
        vars.len()
    );

    let created = M::service_create(&client, &pid, &svc_name).await?;
    let created_id = created.id.clone();
    let service_id = ServiceId::from(created.id);
    println!("  [ACC{}] created {svc_name} = {created_id}", acc.index);

    M::service_instance_set_image(&client, &service_id, &eid, &image).await?;

    let ok = M::variables_upsert_parallel(&client, &pid, &eid, &service_id, &vars).await?;
    println!(
        "  [ACC{}] {svc_name}: {ok}/{} vars upserted",
        acc.index,
        vars.len()
    );

    let deploy_id = M::service_redeploy(&client, &service_id, &eid).await?;
    println!("  [ACC{}] {svc_name}: redeploy {deploy_id}", acc.index);

    let token_fp = client.token_fingerprint();
    let hash = RailwayHash::seal("batch-deploy", &pid, Some(&service_id), &token_fp);
    let line = ExperienceLine::from_hash(
        "GENERAL",
        "BatchDeployer",
        "#81",
        &format!("batch-deploy {svc_name} image={image} acc={}", acc.index),
        "OK",
        "PUSH",
        &hash,
    )?;
    let exp_path = append_line(&root.join(".trinity"), &line).await?;
    tracing::info!(experience = %exp_path.display(), "batch-deploy triplet sealed");

    Ok(())
}

impl Clone for AccountSlot {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            token: self.token.clone(),
            project_id: self.project_id.clone(),
            environment_id: self.environment_id.clone(),
            token_kind: self.token_kind.clone(),
            service_count: self.service_count,
        }
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

/// Fetch one account's project view and return a [`SnapshotAccount`].
async fn snapshot_one_account(
    alias: &str,
    label: &str,
    token_env: &str,
    project_env: &str,
    token_kind_env: &str,
    email: Option<&str>,
) -> SnapshotAccount {
    let token = std::env::var(token_env).ok();
    let project = std::env::var(project_env).ok();

    let (project_id, project_name, services) = if let (Some(tok), Some(proj)) = (token, project) {
        let auth = if let Some(kind) = (!token_kind_env.is_empty())
            .then(|| std::env::var(token_kind_env).ok())
            .flatten()
        {
            match kind.as_str() {
                "team" | "bearer" | "personal" => AuthMode::Team,
                "project" => AuthMode::Project,
                _ if is_uuid_like(&tok) => AuthMode::Project,
                _ => AuthMode::Team,
            }
        } else if is_uuid_like(&tok) {
            AuthMode::Project
        } else {
            AuthMode::Team
        };
        let client = match Client::with_token_and_mode(&tok, auth) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(alias, label, error = %e, "client build failed");
                return SnapshotAccount {
                    alias: alias.to_string(),
                    project_label: label.to_string(),
                    email: email.map(String::from),
                    token_secret: token_env.to_string(),
                    project_id: None,
                    project_name: None,
                    environments: Vec::new(),
                    services: Vec::new(),
                    service_count: 0,
                };
            }
        };
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
                (Some(pv.id.clone()), Some(pv.name.clone()), svcs)
            }
            Err(e) => {
                tracing::warn!(alias, label, error = %e, "skipping account");
                (None, None, Vec::new())
            }
        }
    } else {
        tracing::warn!(alias, label, "missing token_env or project_env in process env");
        (None, None, Vec::new())
    };

    let count = services.len();
    SnapshotAccount {
        alias: alias.to_string(),
        project_label: label.to_string(),
        email: email.map(String::from),
        token_secret: token_env.to_string(),
        project_id,
        project_name,
        environments: Vec::new(),
        services,
        service_count: count,
    }
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
    let mut total_services: usize = 0;

    for spec in &accounts {
        let kv = parse_kv_list(spec);
        let alias = kv.get("alias").cloned().unwrap_or_else(|| "?".to_string());
        let label = kv.get("label").cloned().unwrap_or_else(|| "?".to_string());
        let token_env = kv.get("token_env").cloned().unwrap_or_default();
        let project_env = kv.get("project_env").cloned().unwrap_or_default();
        let token_kind_env = kv.get("token_kind_env").cloned().unwrap_or_default();
        let email = email_map.get(&alias).map(String::as_str);

        let acc = snapshot_one_account(
            &alias, &label, &token_env, &project_env, &token_kind_env, email,
        )
        .await;
        total_services += acc.service_count;
        acc_out.push(acc);
    }

    let total_accounts = acc_out
        .iter()
        .map(|a| a.alias.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let total_projects = acc_out.len();

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

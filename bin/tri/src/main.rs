//! `tri` — thin shim CLI for IGLA project operations.
//!
//! This CLI is a minimal wrapper that delegates to the public crate APIs:
//! - tri-core: deploy, kill, rotate, snapshot, fleet_list
//! - tri-hunt: seed hunter operations
//! - tri-exp: EXP_ID sequence management
//! - tri-canon: name validation
//! - tri-ledger: audit ledger operations
//!
//! Anchor: `phi^2 + phi^-2 = 3`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tri_core::{DeployConfig, ServiceId};
use tri_hunt::SmokeRaceConfig;
use tri_exp::NeonConfig;
use tri_ledger::{LedgerConfig, LedgerRow};

const DEFAULT_PROJECT_ID: &str = "e4fe33bb-3b09-4842-9782-7d2dea1abc9b";
const DEFAULT_ENV_ID: &str = "54e293b9-00a9-4102-814d-db151636d96e";

#[derive(Parser, Debug)]
#[command(
    name = "tri",
    version,
    about = "IGLA project operations CLI",
    long_about = "tri: thin shim for IGLA project operations.\n\
                  All business logic lives in tri-core, tri-hunt, tri-exp, tri-canon, tri-ledger crates."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Core service operations (tri-core)
    Service {
        #[command(subcommand)]
        sub: ServiceCmd,
    },

    /// Seed hunter operations (tri-hunt)
    Hunt {
        #[command(subcommand)]
        sub: HuntCmd,
    },

    /// Experience ID operations (tri-exp)
    Exp {
        #[command(subcommand)]
        sub: ExpCmd,
    },

    /// Name validation and canonicalization (tri-canon)
    Canon {
        #[command(subcommand)]
        sub: CanonCmd,
    },

    /// Audit ledger operations (tri-ledger)
    Ledger {
        #[command(subcommand)]
        sub: LedgerCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ServiceCmd {
    /// Deploy a new service
    Deploy {
        /// Project ID
        #[arg(long, env = "TRIOS_PROJECT", default_value = DEFAULT_PROJECT_ID)]
        project: String,
        /// Environment ID
        #[arg(long, env = "TRIOS_ENV", default_value = DEFAULT_ENV_ID)]
        environment: String,
        /// Service name
        #[arg(long)]
        name: String,
        /// Docker image
        #[arg(long, default_value = "ghcr.io/ghashtag/trios-train-seed:latest")]
        image: String,
        /// Environment variables (KEY=VALUE)
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
        /// Existing service ID to reuse
        #[arg(long)]
        existing: Option<String>,
    },

    /// Delete a service
    Kill {
        /// Service ID
        #[arg(long)]
        service: String,
        /// Confirm deletion
        #[arg(long)]
        yes: bool,
    },

    /// Redeploy a service
    Rotate {
        /// Environment ID
        #[arg(long, env = "TRIOS_ENV", default_value = DEFAULT_ENV_ID)]
        environment: String,
        /// Service ID
        #[arg(long)]
        service: String,
    },

    /// Create fleet snapshot
    Snapshot {
        /// Project ID
        #[arg(long, env = "TRIOS_PROJECT", default_value = DEFAULT_PROJECT_ID)]
        project: String,
    },

    /// List all services
    List {
        /// Project ID
        #[arg(long, env = "TRIOS_PROJECT", default_value = DEFAULT_PROJECT_ID)]
        project: String,
    },
}

#[derive(Subcommand, Debug)]
enum HuntCmd {
    /// Get seed hunter status
    Status,

    /// Run smoke race
    Race {
        /// Number of seeds
        #[arg(long, default_value = "10")]
        count: usize,
        /// Target BPB
        #[arg(long, default_value = "1.85")]
        target_bpb: f64,
        /// Timeout per seed (seconds)
        #[arg(long, default_value = "3600")]
        timeout: u64,
    },

    /// Get rung schedule
    Schedule {
        /// Target BPB
        #[arg(long, default_value = "1.85")]
        target_bpb: f64,
        /// Number of rungs
        #[arg(long, default_value = "10")]
        rungs: i32,
    },

    /// Prune diverging seeds
    Prune {
        /// Expected BPB threshold
        #[arg(long, default_value = "2.0")]
        expected_bpb: f64,
        /// Seed list (comma-separated)
        #[arg(long)]
        seeds: String,
    },
}

#[derive(Subcommand, Debug)]
enum ExpCmd {
    /// Get next EXP_ID
    Next {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
    },

    /// Claim batch of EXP_IDs
    Claim {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
        /// Number of IDs to claim
        #[arg(long, default_value = "10")]
        count: usize,
    },

    /// Peek current EXP_ID (without advancing)
    Peek {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
    },
}

#[derive(Subcommand, Debug)]
enum CanonCmd {
    /// Validate a name
    Validate {
        /// Name to validate
        #[arg(long)]
        name: String,
    },

    /// Validate a name for deployment
    ValidateDeploy {
        /// Name to validate
        #[arg(long)]
        name: String,
    },

    /// Canonicalize a name
    Canonicalize {
        /// Name to canonicalize
        #[arg(long)]
        name: String,
    },

    /// Extract seed from name
    ExtractSeed {
        /// Service name
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum LedgerCmd {
    /// Append seed result
    Append {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
        /// Seed number
        #[arg(long)]
        seed: i32,
        /// BPB value
        #[arg(long)]
        bpb: f64,
        /// Image digest
        #[arg(long)]
        digest: Option<String>,
    },

    /// Query all seed results
    Query {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
    },

    /// Run migrations
    Migrate {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
    },

    /// Verify append-only enforcement
    Verify {
        /// Neon connection string
        #[arg(long, env = "NEON_CONNECTION_STRING")]
        connection: String,
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
        Cmd::Service { sub } => run_service(sub).await?,
        Cmd::Hunt { sub } => run_hunt(sub).await?,
        Cmd::Exp { sub } => run_exp(sub).await?,
        Cmd::Canon { sub } => run_canon(sub)?,
        Cmd::Ledger { sub } => run_ledger(sub).await?,
    }

    Ok(())
}

async fn run_service(cmd: ServiceCmd) -> Result<()> {
    let client = trios_railway_core::Client::from_env()?;

    match cmd {
        ServiceCmd::Deploy {
            project,
            environment,
            name,
            image,
            vars,
            existing,
        } => {
            let project_id = trios_railway_core::ProjectId::from(project);
            let environment_id = trios_railway_core::EnvironmentId::from(environment);
            let existing_service_id = existing.map(trios_railway_core::ServiceId::from);

            let parsed_vars: Vec<(String, String)> = vars
                .iter()
                .filter_map(|v| v.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            let config = DeployConfig {
                project_id,
                environment_id,
                name,
                image,
                vars: parsed_vars,
                existing_service_id,
            };

            let result = tri_core::deploy(&client, config).await?;
            println!("deployed: service_id={} deploy_id={}", result.service_id, result.deploy_id);
        }

        ServiceCmd::Kill { service, yes } => {
            if !yes {
                anyhow::bail!("refusing to delete without --yes");
            }
            let service_id = ServiceId::from(service.clone());
            tri_core::kill(&client, &service_id).await?;
            println!("killed: {service}");
        }

        ServiceCmd::Rotate { environment, service } => {
            let environment_id = trios_railway_core::EnvironmentId::from(environment);
            let service_id = ServiceId::from(service);
            let deploy_id = tri_core::rotate(&client, &service_id, &environment_id).await?;
            println!("rotated: deploy_id={}", deploy_id);
        }

        ServiceCmd::Snapshot { project } => {
            let project_id = trios_railway_core::ProjectId::from(project);
            let snapshot = tri_core::snapshot(&client, &project_id).await?;
            println!("project: {} ({})", snapshot.project_name, snapshot.project_id);
            println!("services: {}", snapshot.services.len());
            for svc in &snapshot.services {
                println!("  {}  {}  {}", svc.id, svc.name, svc.created_at);
            }
        }

        ServiceCmd::List { project } => {
            let project_id = trios_railway_core::ProjectId::from(project);
            let services = tri_core::fleet_list(&client, &project_id).await?;
            println!("services: {}", services.len());
            for svc in &services {
                println!("  {}  {}  {}", svc.id, svc.name, svc.created_at);
            }
        }
    }

    Ok(())
}

async fn run_hunt(cmd: HuntCmd) -> Result<()> {
    match cmd {
        HuntCmd::Status => {
            let status = tri_hunt::seed_hunter_status();
            println!("state: {:?}", status.state);
            println!("seeds: {}", status.seeds.len());
            println!("current_rung: {}", status.schedule.current_rung);
        }

        HuntCmd::Race {
            count,
            target_bpb,
            timeout,
        } => {
            let config = SmokeRaceConfig {
                count,
                target_bpb,
                timeout_seconds: timeout,
            };
            let result = tri_hunt::smoke_race(config).await?;
            println!("duration: {}s", result.duration_seconds);
            if let Some(winner) = result.winner {
                println!("winner: seed={} bpb={:?}", winner.seed, winner.best_bpb);
            }
        }

        HuntCmd::Schedule { target_bpb, rungs } => {
            let schedule = tri_hunt::rung_schedule(target_bpb, rungs);
            println!("rungs: {}", schedule.rungs.len());
            for rung in &schedule.rungs {
                println!("  rung {}: bpb_threshold={} seeds={}", rung.level, rung.bpb_threshold, rung.seeds.len());
            }
        }

        HuntCmd::Prune {
            expected_bpb,
            seeds,
        } => {
            let seed_list: Vec<i32> = seeds
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            let seed_statuses: Vec<tri_hunt::SeedStatus> = seed_list
                .iter()
                .map(|&s| tri_hunt::SeedStatus {
                    seed: s,
                    state: tri_hunt::SeedState::Completed,
                    discovered_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    best_bpb: Some(expected_bpb + 0.5), // Simulate divergence
                })
                .collect();
            let to_prune = tri_hunt::prune_diverging(&seed_statuses, expected_bpb);
            println!("prune {} seeds: {:?}", to_prune.len(), to_prune);
        }
    }

    Ok(())
}

async fn run_exp(cmd: ExpCmd) -> Result<()> {
    match cmd {
        ExpCmd::Next { connection } => {
            let config = NeonConfig { connection_string: connection };
            let result = tri_exp::next_exp_id(&config).await?;
            println!("EXP_ID: {} at {}", result.exp_id, result.allocated_at);
        }

        ExpCmd::Claim { connection, count } => {
            let config = NeonConfig { connection_string: connection };
            let results = tri_exp::claim_exp_ids(&config, count).await?;
            println!("claimed {} EXP_IDs:", results.len());
            for r in &results {
                println!("  {} at {}", r.exp_id, r.allocated_at);
            }
        }

        ExpCmd::Peek { connection } => {
            let config = NeonConfig { connection_string: connection };
            let exp_id = tri_exp::peek_exp_id(&config).await?;
            println!("current EXP_ID: {}", exp_id);
        }
    }

    Ok(())
}

fn run_canon(cmd: CanonCmd) -> Result<()> {
    match cmd {
        CanonCmd::Validate { name } => {
            match tri_canon::validate(&name) {
                tri_canon::ValidationResult::Valid => println!("valid"),
                tri_canon::ValidationResult::Invalid(reason) => {
                    println!("invalid: {}", reason);
                    std::process::exit(1);
                }
            }
        }

        CanonCmd::ValidateDeploy { name } => {
            match tri_canon::validate_for_deploy(&name) {
                tri_canon::ValidationResult::Valid => println!("valid for deploy"),
                tri_canon::ValidationResult::Invalid(reason) => {
                    println!("invalid for deploy: {}", reason);
                    std::process::exit(1);
                }
            }
        }

        CanonCmd::Canonicalize { name } => {
            match tri_canon::canonicalize(&name) {
                Ok(canonical) => println!("{}", canonical),
                Err(e) => {
                    println!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        CanonCmd::ExtractSeed { name } => {
            match tri_canon::extract_seed(&name) {
                Some(seed) => println!("seed: {}", seed),
                None => {
                    println!("no seed found in name");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

async fn run_ledger(cmd: LedgerCmd) -> Result<()> {
    match cmd {
        LedgerCmd::Append {
            connection,
            seed,
            bpb,
            digest,
        } => {
            let config = LedgerConfig { connection_string: connection };
            let row = LedgerRow {
                seed,
                bpb,
                canonical_image_digest: digest,
            };
            let result = tri_ledger::append(&config, &row).await?;
            println!("appended: row_id={} at {}", result.row_id, result.timestamp);
        }

        LedgerCmd::Query { connection } => {
            let config = LedgerConfig { connection_string: connection };
            let rows = tri_ledger::query_all(&config).await?;
            println!("ledger rows: {}", rows.len());
            for row in &rows {
                println!(
                    "  seed={} bpb={} digest={:?}",
                    row.seed,
                    row.bpb,
                    row.canonical_image_digest
                );
            }
        }

        LedgerCmd::Migrate { connection } => {
            let config = LedgerConfig { connection_string: connection };
            tri_ledger::migrate(&config).await?;
            println!("migration complete");
        }

        LedgerCmd::Verify { connection } => {
            let config = LedgerConfig { connection_string: connection };
            let enforced = tri_ledger::verify_append_only(&config).await?;
            println!("append-only enforced: {}", enforced);
        }
    }

    Ok(())
}

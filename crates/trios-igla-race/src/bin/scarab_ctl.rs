//! Scarab Strategy Control Utility
//!
//! Rust CLI для управления scarab стратегиями в Railway PostgreSQL.
//! Заменяет SQL скрипты и bash-скрипты.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio_postgres::NoTls;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Применить миграцию для создания scarab_strategy таблиц
    Migrate,
    /// Создать тестовую стратегию
    CreateTest,
    /// Показать все стратегии
    List,
    /// Показать конкретную стратегию
    Get {
        service_id: String,
    },
    /// Установить статус scarab
    SetStatus {
        service_id: String,
        status: String,
    },
    /// Удалить стратегию
    Delete {
        service_id: String,
    },
}

const MIGRATION_SQL: &str = include_str!("../../../migrations/0007_scarab_strategy.sql");

async fn connect() -> Result<tokio_postgres::Client> {
    let dsn = std::env::var("DATABASE_URL")
        .context("DATABASE_URL не задан")?;

    let (client, conn) = tokio_postgres::connect(&dsn, NoTls).await
        .context("Ошибка подключения к PostgreSQL")?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Ошибка подключения: {e}");
        }
    });

    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "scarab_ctl=info,tokio_postgres=warn".to_string()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Migrate => run_migrate().await?,
        Commands::CreateTest => run_create_test().await?,
        Commands::List => run_list().await?,
        Commands::Get { service_id } => run_get(service_id).await?,
        Commands::SetStatus { service_id, status } => run_set_status(service_id, status).await?,
        Commands::Delete { service_id } => run_delete(service_id).await?,
    }

    Ok(())
}

async fn run_migrate() -> Result<()> {
    println!("📦 Применение миграции scarab_strategy...");

    let client = connect().await?;

    for stmt in MIGRATION_SQL.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }

        if let Err(e) = client.execute(stmt, &[]).await {
            eprintln!("Ошибка выполнения SQL: {e}");
            eprintln!("SQL: {stmt}");
            return Err(e.into());
        }
    }

    println!("✓ Миграция применена успешно!");

    // Проверяем, что таблицы созданы
    let tables = client.query(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE '%scarab%'",
        &[]
    ).await?;

    for row in &tables {
        let name: String = row.get(0);
        println!("  ✓ Таблица: {name}");
    }

    Ok(())
}

async fn run_create_test() -> Result<()> {
    println!("🪲 Создание тестовой стратегии...");

    let client = connect().await?;

    // Комбинации для тестирования: все форматы + все оптимизаторы
    let test_combinations = [
        // fp32 + adamw
        ("scarab-test-fp32-adamw", "fp32", "adamw"),
        // fp32 + muon
        ("scarab-test-fp32-muon", "fp32", "muon"),
        // bf16 + adamw
        ("scarab-test-bf16-adamw", "bf16", "adamw"),
        // gf16 + muon
        ("scarab-test-gf16-muon", "gf16", "muon"),
    ];

    for (service_id, format, optimizer) in test_combinations {
        let sql = "INSERT INTO public.scarab_strategy \
            (service_id, optimizer, hidden, lr, seed, steps, ctx, format, status, description) \
            VALUES ($1, $2, 512, 0.002, 42, 500, 12, $3, 'active', $4) \
            ON CONFLICT (service_id) DO UPDATE SET \
                optimizer = EXCLUDED.optimizer, \
                format = EXCLUDED.format, \
                status = EXCLUDED.status, \
                last_updated = NOW()";

        let description = format!("Test: {} + {}", format, optimizer);

        if let Err(e) = client.execute(sql, &[&service_id, &optimizer, &format, &description]).await {
            eprintln!("  ✗ {service_id}: {e}");
        } else {
            println!("  ✓ {service_id}: {format} + {optimizer}");
        }
    }

    println!("✓ Тестовые стратегии созданы!");
    println!();
    println!("Для запуска scarab локально:");
    println!("  SCARAB_SERVICE_ID=scarab-test-fp32-adamw \\");
    println!("  DATABASE_URL=$DATABASE_URL \\");
    println!("  scarab_strategy");

    Ok(())
}

async fn run_list() -> Result<()> {
    println!("📋 Список scarab стратегий:");

    let client = connect().await?;

    let rows = client.query(
        "SELECT service_id, optimizer, hidden, lr, seed, steps, ctx, format, status, \
               last_updated, strategy_hash, description \
        FROM public.scarab_strategy ORDER BY service_id",
        &[]
    ).await?;

    if rows.is_empty() {
        println!("  (нет стратегий)");
    }

    for row in &rows {
        let service_id: String = row.get(0);
        let optimizer: String = row.get(1);
        let hidden: i32 = row.get(2);
        let lr: f64 = row.get(3);
        let format: String = row.get(7);
        let status: String = row.get(8);
        let hash: String = row.get(9);

        let status_icon = match status.as_str() {
            "active" => "🟢",
            "paused" => "⏸️",
            "disabled" => "⚫",
            _ => "❓",
        };

        println!("  {status_icon} {} | {} | hidden={} lr={} fmt={}",
                 service_id, optimizer, hidden, lr, format);

        if hash.len() > 12 {
            println!("       hash: {}...", &hash[..12]);
        } else {
            println!("       hash: {}", hash);
        }
    }

    Ok(())
}

async fn run_get(service_id: String) -> Result<()> {
    println!("🔍 Детали стратегии: {}", service_id);

    let client = connect().await?;

    let row = client.query_opt(
        "SELECT service_id, optimizer, hidden, lr, seed, steps, ctx, format, \
               status, strategy_hash, last_updated, description, canon_name \
        FROM public.scarab_strategy WHERE service_id = $1",
        &[&service_id],
    ).await?;

    match row {
        Some(r) => {
            let optimizer: String = r.get(1);
            let hidden: i32 = r.get(2);
            let lr: f64 = r.get(3);
            let format: String = r.get(7);
            let status: String = r.get(8);
            let hash: String = r.get(9);
            let last_updated = r.get::<_, chrono::DateTime<chrono::Utc>>(10);

            println!("  Service ID: {}", r.get::<_, String>(0));
            println!("  Optimizer: {}", optimizer);
            println!("  Hidden: {}", hidden);
            println!("  LR: {}", lr);
            println!("  Steps: {}", r.get::<_, i32>(5));
            println!("  Context: {}", r.get::<_, i32>(6));
            println!("  Format: {}", format);
            println!("  Seed: {}", r.get::<_, i64>(4));
            println!("  Status: {}", status);
            println!("  Updated: {}", last_updated);
            println!("  Hash: {}", hash);
        }
        None => {
            eprintln!("✗ Стратегия не найдена: {}", service_id);
            return Err(anyhow::anyhow!("Стратегия не найдена"));
        }
    }

    Ok(())
}

async fn run_set_status(service_id: String, status: String) -> Result<()> {
    if !matches!(status.as_str(), "active" | "paused" | "disabled") {
        eprintln!("✗ Неверный статус. Используйте: active, paused, disabled");
        return Err(anyhow::anyhow!("Неверный статус"));
    }

    println!("⚙️  Установка статуса {} для {}", status, service_id);

    let client = connect().await?;

    let rows_affected = client.execute(
        "UPDATE public.scarab_strategy SET status = $1, last_updated = NOW() WHERE service_id = $2",
        &[&status, &service_id],
    ).await?;

    if rows_affected > 0 {
        println!("✓ Статус обновлен");
    } else {
        eprintln!("✗ Стратегия не найдена");
    }

    Ok(())
}

async fn run_delete(service_id: String) -> Result<()> {
    println!("🗑️  Удаление стратегии: {}", service_id);

    let client = connect().await?;

    let rows_affected = client.execute(
        "DELETE FROM public.scarab_strategy WHERE service_id = $1",
        &[&service_id],
    ).await?;

    if rows_affected > 0 {
        println!("✓ Стратегия удалена");
    } else {
        eprintln!("✗ Стратегия не найдена");
    }

    Ok(())
}

//! Neon connection helpers — strip channel_binding (rustls limitation, per
//! [trios-trainer-igla#84](https://github.com/gHashTag/trios-trainer-igla/issues/84)
//! round 3) and install the rustls CryptoProvider exactly once.

use rustls::ClientConfig;
use std::sync::OnceLock;
use tokio_postgres::{Client, Error};
use tokio_postgres_rustls::MakeRustlsConnect;

/// Default boevoi DSN-A (single source of truth for IGLA RACE / WAVE-GF-001).
pub const DEFAULT_DSN: &str = "postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require";

/// Strip a single `channel_binding=...` token from the URI query string.
/// Idempotent. Returns the input verbatim if no such token is present.
pub fn strip_channel_binding(dsn: &str) -> String {
    let Some(qpos) = dsn.find('?') else {
        return dsn.to_string();
    };
    let (head, query) = dsn.split_at(qpos + 1);
    let kept: Vec<&str> = query
        .split('&')
        .filter(|kv| !kv.trim_start().starts_with("channel_binding="))
        .collect();
    let rebuilt = kept.join("&");
    if rebuilt.is_empty() {
        head.trim_end_matches('?').to_string()
    } else {
        format!("{head}{rebuilt}")
    }
}

fn ensure_crypto_provider() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn make_tls() -> MakeRustlsConnect {
    ensure_crypto_provider();
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    MakeRustlsConnect::new(cfg)
}

/// Connect to Neon, returning a `tokio_postgres::Client`. Spawns the underlying
/// connection task. DSN's `channel_binding` (incompatible with rustls 0.23) is
/// stripped automatically.
pub async fn connect(dsn: &str) -> Result<Client, Error> {
    let stripped = strip_channel_binding(dsn);
    let tls = make_tls();
    let (client, conn) = tokio_postgres::connect(&stripped, tls).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[igla-ops::neon] connection error: {e}");
        }
    });
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strip_cb_removes_only_that_param() {
        assert_eq!(
            strip_channel_binding("postgresql://u:p@h/db?sslmode=require&channel_binding=require"),
            "postgresql://u:p@h/db?sslmode=require"
        );
    }
    #[test]
    fn strip_cb_passthrough() {
        assert_eq!(
            strip_channel_binding("postgresql://u:p@h/db?sslmode=require"),
            "postgresql://u:p@h/db?sslmode=require"
        );
    }
    #[test]
    fn strip_cb_no_query() {
        assert_eq!(
            strip_channel_binding("postgresql://u:p@h/db"),
            "postgresql://u:p@h/db"
        );
    }
}

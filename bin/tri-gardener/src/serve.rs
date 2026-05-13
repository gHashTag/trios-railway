//! Issue #55: `tri-gardener serve --interval=N` long-lived runner.
//!
//! Replaces dependence on Railway's (absent) scheduled-restart with an
//! in-process `tokio::time::interval` driver. SIGTERM / SIGINT triggers
//! graceful shutdown after the in-flight tick completes.
//!
//! Drift-free: `Interval::tick` with `MissedTickBehavior::Delay` skips
//! ahead instead of catching up, so a slow tick (e.g. Neon stall) does
//! not produce a thundering herd.

use anyhow::{anyhow, Result};
use std::time::Duration;

/// Validation bounds.
pub const MIN_INTERVAL_SECS: u64 = 60;
pub const MAX_INTERVAL_SECS: u64 = 86_400;

/// Validate the interval value an operator passed on the CLI.
pub fn validate_interval(secs: u64) -> Result<Duration> {
    if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&secs) {
        return Err(anyhow!(
            "interval {secs}s out of range [{MIN_INTERVAL_SECS}, {MAX_INTERVAL_SECS}]"
        ));
    }
    Ok(Duration::from_secs(secs))
}

/// Trait abstracting the per-tick body so unit tests can count
/// invocations without spinning up the real loop_once_live.
#[async_trait::async_trait]
pub trait TickHandler: Send + Sync {
    async fn on_tick(&self, tick_index: u64) -> Result<()>;
}

/// Run `handler` every `interval` until `should_stop` returns true. The
/// caller owns the stop-flag (typically driven by a SIGINT/SIGTERM
/// handler in `main.rs`).
pub async fn serve_loop(
    interval: Duration,
    handler: &dyn TickHandler,
    should_stop: impl Fn() -> bool + Send + Sync,
) -> Result<u64> {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut idx: u64 = 0;
    loop {
        if should_stop() {
            tracing::info!(idx, "serve_loop: stop requested, exiting cleanly");
            return Ok(idx);
        }
        ticker.tick().await;
        if should_stop() {
            tracing::info!(idx, "serve_loop: stop after tick wait, exiting");
            return Ok(idx);
        }
        idx = idx.saturating_add(1);
        if let Err(e) = handler.on_tick(idx).await {
            // R5 honesty: we log per-tick failures but do not crash the
            // serve loop — the next interval is the natural retry.
            tracing::error!(?e, idx, "tick failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    struct CountingHandler {
        count: Arc<AtomicU64>,
    }
    #[async_trait::async_trait]
    impl TickHandler for CountingHandler {
        async fn on_tick(&self, _i: u64) -> Result<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn validate_interval_rejects_too_small() {
        assert!(validate_interval(10).is_err());
    }
    #[test]
    fn validate_interval_rejects_too_large() {
        assert!(validate_interval(99_999).is_err());
    }
    #[test]
    fn validate_interval_accepts_3600() {
        assert!(validate_interval(3600).is_ok());
    }
    #[test]
    fn validate_interval_accepts_min_and_max() {
        assert!(validate_interval(60).is_ok());
        assert!(validate_interval(86_400).is_ok());
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn serve_emits_tick_every_interval() {
        // Contract test: tokio's paused clock advances by 3600s 5 times,
        // and the handler should be called 5 times.
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let stop = Arc::new(AtomicU64::new(0));
        let stop_check = {
            let stop = stop.clone();
            move || stop.load(Ordering::SeqCst) >= 5
        };

        let interval = Duration::from_secs(3600);
        let drive = tokio::spawn(async move { serve_loop(interval, &handler, stop_check).await });

        // Drive the clock forward in 3600s steps and bump the counter
        // so the stop predicate eventually fires.
        for _ in 0..5 {
            tokio::time::advance(Duration::from_secs(3600)).await;
            tokio::task::yield_now().await;
            stop.fetch_add(1, Ordering::SeqCst);
        }
        // One more advance to allow the loop to observe the stop flag.
        tokio::time::advance(Duration::from_secs(3600)).await;

        let res = drive.await.unwrap().unwrap();
        let observed = count.load(Ordering::SeqCst);
        assert!(
            observed >= 4 && observed <= 6,
            "expected ~5 ticks, observed {observed} (final idx {res})"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn serve_loop_exits_when_stop_is_true_at_start() {
        let count = Arc::new(AtomicU64::new(0));
        let handler = CountingHandler {
            count: count.clone(),
        };
        let res = serve_loop(
            Duration::from_secs(60),
            &handler,
            || true, // already stopped
        )
        .await
        .unwrap();
        assert_eq!(res, 0);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}

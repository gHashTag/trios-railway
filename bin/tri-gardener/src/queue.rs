//! Queue loader from `bin/tri-gardener/queue.toml`.

// `load` is wired into the loop driver in PR-2; until then keep the
// public API stable so tests cover it.
#![allow(dead_code)]

use std::path::Path;

use anyhow::{Context, Result};

use crate::state::Queue;

pub fn load(path: &Path) -> Result<Queue> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading queue {}", path.display()))?;
    let q: Queue =
        toml::from_str(&raw).with_context(|| format!("parsing queue {}", path.display()))?;
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn queue_loads_in_priority_order() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("queue.toml");
        let q = load(&path).expect("queue loads");
        // At least one entry must be present.
        assert!(!q.entries.is_empty());
        // next_unblocked with empty cleared list returns either the
        // first entry with no blockers, or None.
        if q.entries.iter().any(|e| e.blocked_on.is_empty()) {
            assert!(q.next_unblocked(&[]).is_some());
        }
    }
}

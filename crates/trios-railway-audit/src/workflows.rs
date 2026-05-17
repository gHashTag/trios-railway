//! ADR-0042 text-level regression guard for GitHub Actions workflows.
//!
//! Scarab fleet push (`variableUpsert` / `serviceInstanceDeployV2` /
//! `serviceInstanceRedeploy` / `serviceDelete` / `serviceInstanceUpdate`)
//! must not be reachable from `schedule`, `push`, `pull_request`,
//! `workflow_run`, or `repository_dispatch` triggers. Every workflow that
//! issues those GraphQL mutations must either be hard-disabled (legacy
//! scarab fleet push) or double-key gated (operator-tier recovery:
//! `confirm=PHI` plus repo secret `LEGACY_PUSH_PATH_ENABLE=1`).
//!
//! These are text-level invariants only — they fire under `cargo test` and
//! prevent silent drift; they do not replace operator review.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn workflows_dir() -> PathBuf {
        // CARGO_MANIFEST_DIR = crates/trios-railway-audit
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join(".github/workflows")
    }

    fn read_workflows() -> BTreeMap<String, String> {
        let dir = workflows_dir();
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(&dir).expect("read .github/workflows") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yml") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("file name")
                .to_string();
            let body = fs::read_to_string(&path).expect("read workflow");
            out.insert(name, body);
        }
        assert!(
            !out.is_empty(),
            "no workflows found under .github/workflows"
        );
        out
    }

    const PUSH_MUTATIONS: &[&str] = &[
        "variableUpsert",
        "serviceInstanceDeployV2",
        "serviceInstanceRedeploy",
        "serviceDelete",
        "serviceInstanceUpdate",
        "serviceCreate",
        "templateDeployV2",
    ];

    fn has_push_mutation(body: &str) -> bool {
        PUSH_MUTATIONS.iter().any(|m| body.contains(m))
    }

    /// Workflows that legitimately exercise Railway push for NON-scarab
    /// operator recovery. Each must carry the ADR-0042 double-key gate
    /// (input `confirm=PHI` plus secret `LEGACY_PUSH_PATH_ENABLE=1`).
    const OPERATOR_RECOVERY_ALLOWLIST: &[&str] = &[
        "mcp-emergency-redeploy.yml",
        "writer-env-fix.yml",
        "deploy-from-template.yml",
        // redeploy-single is the canonical PR-#219 break-glass form.
        "redeploy-single.yml",
    ];

    /// Workflows whose push stages are gated by `LEGACY_PUSH_PATH_ENABLE`
    /// at the *step* level (cron is read-only, dispatch stays review-only
    /// by default). This is the gardener pattern from PR #219.
    const STEP_GATED_ALLOWLIST: &[&str] = &["gardener-loop.yml"];

    /// ADR-0042 §"Closed by this ADR" mentions that the Rust mutation
    /// functions are still compiled. The corresponding workflows that
    /// have been DEPRECATED by `if: false` + a refusing-`exit 1` job
    /// keep the file for git history. These are recognised by the
    /// presence of an `adr0042-disabled` or `[DEPRECATED L-SS7]` /
    /// `[ADR-0042 disabled]` marker.
    fn is_hard_disabled(name: &str, body: &str) -> bool {
        let _ = name;
        let has_marker = body.contains("[ADR-0042 disabled]")
            || body.contains("[DEPRECATED L-SS7]")
            || body.contains("adr0042-disabled");
        let has_block_job = body.contains("LEGACY_PUSH_PATH_DISABLED") && body.contains("exit 1");
        let has_if_false_on_push_job = body.contains("if: false");
        has_marker && (has_block_job || has_if_false_on_push_job)
    }

    /// A workflow with push mutations is classified into one of three
    /// buckets. Anything not in a bucket is a regression.
    enum Bucket {
        OperatorRecovery,
        StepGated,
        HardDisabled,
        Unclassified,
    }

    fn classify(name: &str, body: &str) -> Bucket {
        if OPERATOR_RECOVERY_ALLOWLIST.contains(&name) {
            return Bucket::OperatorRecovery;
        }
        if STEP_GATED_ALLOWLIST.contains(&name) {
            return Bucket::StepGated;
        }
        if is_hard_disabled(name, body) {
            return Bucket::HardDisabled;
        }
        Bucket::Unclassified
    }

    /// Every workflow that contains a Railway push mutation must be
    /// classified into one of the three approved buckets.
    #[test]
    fn every_push_workflow_is_classified() {
        let workflows = read_workflows();
        let mut unclassified = Vec::new();
        for (name, body) in &workflows {
            if !has_push_mutation(body) {
                continue;
            }
            if let Bucket::Unclassified = classify(name, body) {
                unclassified.push(name.clone());
            }
        }
        assert!(
            unclassified.is_empty(),
            "ADR-0042 regression: workflows with Railway push mutations are \
             not classified (must be hard-disabled, step-gated, or \
             operator-recovery double-keyed): {unclassified:?}"
        );
    }

    /// Operator-recovery workflows must enforce BOTH `confirm == 'PHI'`
    /// (input) AND `LEGACY_PUSH_PATH_ENABLE == '1'` (secret) before any
    /// push mutation runs.
    #[test]
    fn operator_recovery_workflows_are_double_keyed() {
        let workflows = read_workflows();
        for name in OPERATOR_RECOVERY_ALLOWLIST {
            let body = workflows
                .get(*name)
                .unwrap_or_else(|| panic!("missing workflow {name}"));
            assert!(
                body.contains("ADR-0042"),
                "{name}: must reference ADR-0042 in workflow header/comment"
            );
            // The double-key guard step does an inverted check
            // `[[ "${{ inputs.confirm }}" != "PHI" ]]` to refuse early —
            // assert that the negation form appears in a `run:` step.
            assert!(
                body.contains("inputs.confirm")
                    && (body.contains("!= \"PHI\"") || body.contains("!= 'PHI'")),
                "{name}: must refuse when inputs.confirm != 'PHI'"
            );
            assert!(
                body.contains("LEGACY_PUSH_PATH_ENABLE"),
                "{name}: must require the LEGACY_PUSH_PATH_ENABLE second key"
            );
        }
    }

    /// No workflow that contains push mutations may be reachable from
    /// `schedule`, `push`, `pull_request`, `workflow_run`, or
    /// `repository_dispatch` triggers. The only triggers allowed are
    /// `workflow_dispatch` (manual) — the gardener-loop schedule is
    /// permitted because its push stages are step-gated and the cron
    /// path never sets the enable env.
    #[test]
    fn no_push_mutation_workflow_has_automatic_trigger() {
        let workflows = read_workflows();
        // gardener-loop runs on schedule but every push step is gated on
        // `env.LEGACY_PUSH_PATH_ENABLE == '1'`, which cron never sets.
        // Verify that pattern is intact.
        let gardener = workflows
            .get("gardener-loop.yml")
            .expect("gardener-loop.yml must exist");
        let gated_steps = gardener
            .matches("env.LEGACY_PUSH_PATH_ENABLE == '1'")
            .count();
        assert!(
            gated_steps >= 5,
            "gardener-loop must keep its push-stage gates (expected >= 5, got {gated_steps})"
        );

        let forbidden_triggers = [
            "push:",
            "pull_request:",
            "workflow_run:",
            "repository_dispatch:",
        ];
        for (name, body) in &workflows {
            if !has_push_mutation(body) {
                continue;
            }
            for trig in &forbidden_triggers {
                assert!(
                    !body.contains(trig),
                    "{name}: push-mutation workflow must not declare an automatic '{trig}' trigger"
                );
            }
            // schedule: is only allowed on gardener-loop (step-gated).
            if name != "gardener-loop.yml" {
                assert!(
                    !body.contains("schedule:"),
                    "{name}: only gardener-loop.yml may carry a schedule trigger \
                     (push stages must be step-gated); see ADR-0042"
                );
            }
        }
    }

    /// Workflows in the hard-disabled bucket must emit an explicit
    /// `exit 1` refuse-step so a `workflow_dispatch` click cannot
    /// silently no-op into a SUCCESS run.
    #[test]
    fn hard_disabled_workflows_refuse_explicitly() {
        let workflows = read_workflows();
        for (name, body) in &workflows {
            if !has_push_mutation(body) {
                continue;
            }
            if !matches!(classify(name, body), Bucket::HardDisabled) {
                continue;
            }
            assert!(
                body.contains("exit 1"),
                "{name}: hard-disabled workflow must `exit 1` from a refuse-step"
            );
            assert!(
                body.contains("ADR-0042"),
                "{name}: hard-disabled workflow must reference ADR-0042"
            );
        }
    }
}

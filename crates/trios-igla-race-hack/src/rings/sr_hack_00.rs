//! SR-HACK-00: Trinity Glossary
//!
//! Structural definitions for all outreach DMs, PR comments, and internal docs.
//! These terms are the foundation of Trinity ring-pattern architecture.

use std::fmt::Display;

/// Trinity architecture term
///
/// Each term represents a core concept in the Trinity system
/// with a defined definition and markdown representation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Term {
    /// O(1) Test-Time Training pipeline
    ///
    /// The end-to-end pipeline that trains models during inference time,
    /// generating synthetic data, training, and evaluating in a single loop.
    PipelineO1,

    /// Algorithm arena entry point
    ///
    /// The spec/verifier wrapper around train_gpt.py payloads that
    /// defines the interface for algorithm submissions.
    AlgorithmEntry,

    /// Execution lane
    ///
    /// A dedicated context for concurrent execution, isolating work
    /// to prevent collisions between agents or processes.
    Lane,

    /// Quality gate
    ///
    /// A verification point in the pipeline where work must pass
    /// validation criteria before proceeding to the next stage.
    Gate,

    /// Ring structural tier
    ///
    /// One of the three GOLD tiers: Pipeline (I), Arena (II), or Hack (III).
    /// Each tier has distinct responsibilities in the architecture.
    RingTier,

    /// Knowledge graph domain
    ///
    /// A bounded domain in the NEON single-writer system containing
    /// chapters and related terminology.
    KINGDOM,

    /// Inverse scope constraint
    ///
    /// A boundary defining what is NOT included in the current scope,
    /// serving as negative constraints on task execution.
    IScope,

    /// Speculative research
    ///
    /// Experimental work exploring new directions without committing
    /// to implementation.
    SR,

    /// Bug report
    ///
    /// A reported issue requiring investigation and potential fix.
    BR,

    /// Inventory entry
    ///
    /// A tracked resource, dependency, or capability in the system.
    INV,

    /// Constitutional laws
    ///
    /// The fundamental rules (L1-L9) governing Trinity agent behavior,
    /// encoding the PHI LOOP workflow and operational constraints.
    LAWS,

    /// Autonomous executor
    ///
    /// An AI agent that executes tasks within the Trinity framework,
    /// bound by LAWS and equipped with codename/soul-name.
    Agent,

    /// Agent codename
    ///
    /// The functional designation of an agent (e.g., ALPHA, BETA, GAMMA),
    /// determining its role and capabilities within the Trinity system.
    Codename,

    /// Agent soul name
    ///
    /// The human-readable identity of an agent (e.g., "Scarab Smith"),
    /// used in commits, heartbeats, and documentation.
    SoulName,

    /// Status heartbeat
    ///
    /// Periodic status report from an agent, encoding execution state,
    /// progress, and any blockers in a structured format.
    HEARTBEAT,

    /// Completion checklist
    ///
    /// The final validation gates an agent must pass before declaring
    /// task completion, ensuring all requirements are met.
    DONE,
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::PipelineO1 => write!(f, "PipelineO1"),
            Term::AlgorithmEntry => write!(f, "AlgorithmEntry"),
            Term::Lane => write!(f, "Lane"),
            Term::Gate => write!(f, "Gate"),
            Term::RingTier => write!(f, "RingTier"),
            Term::KINGDOM => write!(f, "KINGDOM"),
            Term::IScope => write!(f, "I-SCOPE"),
            Term::SR => write!(f, "SR"),
            Term::BR => write!(f, "BR"),
            Term::INV => write!(f, "INV"),
            Term::LAWS => write!(f, "LAWS"),
            Term::Agent => write!(f, "Agent"),
            Term::Codename => write!(f, "Codename"),
            Term::SoulName => write!(f, "Soul-name"),
            Term::HEARTBEAT => write!(f, "HEARTBEAT"),
            Term::DONE => write!(f, "DONE"),
        }
    }
}

impl Term {
    /// Returns the term as markdown for documentation
    pub fn as_markdown(&self) -> String {
        match self {
            Term::PipelineO1 => r#"
**PipelineO1** — O(1) Test-Time Training pipeline

The end-to-end pipeline that trains models during inference time,
generating synthetic data, training, and evaluating in a single loop.
"#.trim().to_string(),

            Term::AlgorithmEntry => r#"
**AlgorithmEntry** — Algorithm arena entry point

The spec/verifier wrapper around train_gpt.py payloads that
defines the interface for algorithm submissions.
"#.trim().to_string(),

            Term::Lane => r#"
**Lane** — Execution lane

A dedicated context for concurrent execution, isolating work
to prevent collisions between agents or processes.
"#.trim().to_string(),

            Term::Gate => r#"
**Gate** — Quality gate

A verification point in the pipeline where work must pass
validation criteria before proceeding to the next stage.
"#.trim().to_string(),

            Term::RingTier => r#"
**RingTier** — Ring structural tier

One of the three GOLD tiers: Pipeline (I), Arena (II), or Hack (III).
Each tier has distinct responsibilities in the architecture.
"#.trim().to_string(),

            Term::KINGDOM => r#"
**KINGDOM** — Knowledge graph domain

A bounded domain in the NEON single-writer system containing
chapters and related terminology.
"#.trim().to_string(),

            Term::IScope => r#"
**I-SCOPE** — Inverse scope constraint

A boundary defining what is NOT included in the current scope,
serving as negative constraints on task execution.
"#.trim().to_string(),

            Term::SR => r#"
**SR** — Speculative research

Experimental work exploring new directions without committing
to implementation.
"#.trim().to_string(),

            Term::BR => r#"
**BR** — Bug report

A reported issue requiring investigation and potential fix.
"#.trim().to_string(),

            Term::INV => r#"
**INV** — Inventory entry

A tracked resource, dependency, or capability in the system.
"#.trim().to_string(),

            Term::LAWS => r#"
**LAWS** — Constitutional laws

The fundamental rules (L1-L9) governing Trinity agent behavior,
encoding the PHI LOOP workflow and operational constraints.
"#.trim().to_string(),

            Term::Agent => r#"
**Agent** — Autonomous executor

An AI agent that executes tasks within the Trinity framework,
bound by LAWS and equipped with codename/soul-name.
"#.trim().to_string(),

            Term::Codename => r#"
**Codename** — Agent codename

The functional designation of an agent (e.g., ALPHA, BETA, GAMMA),
determining its role and capabilities within the Trinity system.
"#.trim().to_string(),

            Term::SoulName => r#"
**Soul-name** — Agent soul name

The human-readable identity of an agent (e.g., "Scarab Smith"),
used in commits, heartbeats, and documentation.
"#.trim().to_string(),

            Term::HEARTBEAT => r#"
**HEARTBEAT** — Status heartbeat

Periodic status report from an agent, encoding execution state,
progress, and any blockers in a structured format.
"#.trim().to_string(),

            Term::DONE => r#"
**DONE** — Completion checklist

The final validation gates an agent must pass before declaring
task completion, ensuring all requirements are met.
"#.trim().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_terms_display() {
        let terms = vec![
            Term::PipelineO1,
            Term::AlgorithmEntry,
            Term::Lane,
            Term::Gate,
            Term::RingTier,
            Term::KINGDOM,
            Term::IScope,
            Term::SR,
            Term::BR,
            Term::INV,
            Term::LAWS,
            Term::Agent,
            Term::Codename,
            Term::SoulName,
            Term::HEARTBEAT,
            Term::DONE,
        ];

        assert_eq!(terms.len(), 16, "Must have exactly 16 terms");

        for term in terms {
            let display = format!("{}", term);
            let markdown = term.as_markdown();

            assert!(!display.is_empty(), "Display should not be empty");
            assert!(!markdown.is_empty(), "Markdown should not be empty");
            assert!(markdown.starts_with("**"), "Markdown should start with **");
        }
    }

    #[test]
    fn test_term_serialization() {
        let term = Term::PipelineO1;

        let json = serde_json::to_string(&term).unwrap();
        let deserialized: Term = serde_json::from_str(&json).unwrap();

        assert_eq!(term, deserialized);
    }

    #[test]
    fn test_soul_name_format() {
        // Ensure hyphenated display for Soul-name
        let soul = format!("{}", Term::SoulName);
        assert_eq!(soul, "Soul-name");
    }
}

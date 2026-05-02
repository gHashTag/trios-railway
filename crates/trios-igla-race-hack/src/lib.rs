//! Trinity Ring-Pattern Architecture — GOLD III: Hack Layer
//!
//! This crate provides outreach, documentation, and community plumbing for
//! the Trinity system. Each ring is a self-contained specification
//! with well-defined terms and interfaces.

pub mod rings;

// Export Term from SR-HACK-00 for convenient use
pub use rings::sr_hack_00::Term;

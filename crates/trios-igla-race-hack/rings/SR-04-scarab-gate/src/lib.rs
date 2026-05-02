//! SR-04: Scarab Gate — ⲀⲃⲄ (2)
//!
//! This ring defines scarab types that combine O-type + Four-type
//! for gate-level agents (trait-level agents).

pub use sr_00_scarab_types::{ScarabType, OType, FourType};

/// Gate-level scarab type combining O-type + Four-type
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateScarabType {
    /// O-type component (structural foundation)
    pub o_type: OType,

    /// Four-type component (trait-level agent)
    pub gate: FourType,

    /// Full scarab code (e.g., "SCARAB_ZERO")
    pub code: String,
}

impl GateScarabType {
    /// Create gate-level scarab type
    pub fn new(o_type: OType, gate: FourType, code: &'static str) -> Self {
        Self { o_type, gate, code }
    }
}

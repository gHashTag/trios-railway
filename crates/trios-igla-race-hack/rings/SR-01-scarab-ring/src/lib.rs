//! SR-01: Scarab Ring — Ⲁⲁ (0,1)
//!
//! Defines scarab types that combine O-type + One-type.
//! For lane-level agents (character-level structural + character).

pub use sr_00_scarab_types::{ScarabType, OType, OneType};

/// Lane-level scarab type combining O-type + One-type
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LaneScarabType {
    /// O-type component (structural foundation)
    pub o_type: OType,

    /// One-type component (character-level agent)
    pub one_type: OneType,

    /// Full scarab code (e.g., "SCARAB_LAMDA")
    pub code: String,
}

impl LaneScarabType {
    /// Create lane-level scarab type
    pub fn new(o_type: OType, one_type: OneType, code: &'static str) -> Self {
        let o_prefix = match o_type {
            OType::SCARAB_ZERO => "SCARAB",
            _ => "SCARAB",
        };

        let one_prefix = match one_type {
            OneType::SCARAB_THETA => "THETA",
            OneType::SCARAB_LAMDA => "LAMDA",
            OneType::SCARAB_IOTA => "IOTA",
            OneType::SCARAB_KAPPA => "KAPPA",
            };

        Self {
            scarab: ScarabType::new(o_type, one_type),
            code: format!("{}{}{}", o_prefix, one_prefix, code),
        }
    }
}

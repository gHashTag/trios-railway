//! SR-02: Scarab Lane — ⲁⲄⲃ (2)
//!
//! This ring defines scarab types that combine O-type + Two-type
//! for lane-level agents (character-level agents).

pub use sr_00_scarab_types::{ScarabType, OType, OneType};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LaneScarabType {
    /// Combined scarab type
    pub scarab: ScarabType,

    /// Two-type component (character-level agent)
    pub lane: OneType,

    /// Full scarab code (e.g., "SCARAB_LAMDA")
    pub code: String,
}

impl LaneScarabType {
    /// Create lane-level scarab type
    pub fn new(o_type: OType, lane: OneType, code: &'static str) -> Self {
        let o_prefix = match o_type {
            OType::SCARAB_ZERO => "SCARAB",
            _ => "SCARAB",
        };

        let l_prefix = match lane {
            OneType::SCARAB_THETA => "THETA",
            _ => "LAMDA",
        };

        Self {
            scarab: ScarabType::new(o_type, lane),
            code: format!("{}{}", o_prefix, l_prefix, code),
        }
    }
}

//! SR-03: Scarab Soul — ⲄⲃⲄ (2)
//!
//! This ring defines scarab types that combine O-type + Three-type
//! for soul-level agents (character-level agents).

pub use sr_00_scarab_types::{ScarabType, OType, ThreeType};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoulScarabType {
    /// Combined scarab type
    pub scarab: ScarabType,

    /// Three-type component (soul-level agent)
    pub soul: ThreeType,

    /// Full scarab code (e.g., "SCARAB_LAMDA")
    pub code: String,
}

impl SoulScarabType {
    /// Create soul-level scarab type
    pub fn new(o_type: OType, soul: ThreeType, code: &'static str) -> Self {
        Self {
            scarab: ScarabType::new(o_type, soul, code),
        }
    }
}

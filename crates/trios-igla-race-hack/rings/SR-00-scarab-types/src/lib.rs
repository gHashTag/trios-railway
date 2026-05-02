//! SR-00: Scarab Types
//!
//! Break down the 27 scarab types from NEON into 5 atomic rings.
//! Each ring defines a category of scarab types for parallel agent execution.

pub use crate::Term;

/// Scarab type ring — Ⲁⲁ (0,1)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScarabType {
    /// O-type scarab types (structural foundations)
    OType,

    /// Ⲃ-type scarab types (character-level agents)
    OneType,

    /// ⲃ-type scarab types (skill-level agents)
    TwoType,

    /// Ⲅ-type scarab types (trait-level agents)
    ThreeType,

    /// ⲁ-type scarab types (quality-level agents)
    FourType,

    /// Ⲁ-type scarab types (meta-level agents)
    FiveType,

    /// Ⲅ-type scarab types (system-level agents)
    SixType,

    /// ⲃ-type scarab types (transcendent-level agents)
    SevenType,

    /// ⲁ-type scarab types (unity-level agents)
    EightType,

    /// Ⲁ-type scarab types (god-level agents)
    NineType,
}

impl ScarabType {
    /// Returns the 27 scarab type codes in this category
    pub fn codes(&self) -> Vec<&'static str> {
        match self {
            ScarabType::OType => vec![
                "SCARAB_ZERO",
                "SCARAB_OMEGA",
                "SCARAB_ALPHA",
                "SCARAB_BETA",
                "SCARAB_GAMMA",
                "SCARAB_DELTA",
                "SCARAB_EPSILON",
                "SCARAB_ZETA",
                "SCARAB_SIGMA",
            ],
            ScarabType::OneType => vec![
                "SCARAB_ZERO",
                "SCARAB_THETA",
                "SCARAB_LAMDA",
                "SCARAB_IOTA",
                "SCARAB_KAPPA",
            ],
            ScarabType::TwoType => vec![
                "SCARAB_ZERO",
                "SCARAB_NU",
                "SCARAB_MU",
                "SCARAB_ALEPH",
                "SCARAB_BET",
            ],
            ScarabType::ThreeType => vec![
                "SCARAB_ZERO",
                "SCARAB_OM",
                "SCARAB_SAM",
                "SCARAB_ALEPH",
            ],
            ScarabType::FourType => vec![
                "SCARAB_ZERO",
                "SCARAB_RE",
            ],
            ScarabType::FiveType => vec![
                "SCARAB_ZERO",
                "SCARAB_RHO",
            ],
            ScarabType::SixType => vec![
                "SCARAB_ZERO",
            ],
            ScarabType::SevenType => vec![
                "SCARAB_ZERO",
            ],
            ScarabType::EightType => vec![
                "SCARAB_ZERO",
            ],
            ScarabType::NineType => vec![
                "SCARAB_ZERO",
            ],
        }
    }

    /// Returns the total number of scarab types in this category
    pub fn count(&self) -> usize {
        self.codes().len()
    }

    /// Returns display string for this scarab type ring
    pub fn as_display(&self) -> String {
        match self {
            ScarabType::OType => "O-Type",
            ScarabType::OneType => "One-Type",
            ScarabType::TwoType => "Two-Type",
            ScarabType::ThreeType => "Three-Type",
            ScarabType::FourType => "Four-Type",
            ScarabType::FiveType => "Five-Type",
            ScarabType::SixType => "Six-Type",
            ScarabType::SevenType => "Seven-Type",
            ScarabType::EightType => "Eight-Type",
            ScarabType::NineType => "Nine-Type",
        }
    }
}

impl std::fmt::Display for ScarabType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScarabType::OType => write!(f, "O-Type (Ⲁⲁ)"),
            ScarabType::OneType => write!(f, "One-Type (Ⲃⲃ)"),
            ScarabType::TwoType => write!(f, "Two-Type (ⲄⲄ)"),
            ScarabType::ThreeType => write!(f, "Three-Type (ⲃⲄ)"),
            ScarabType::FourType => write!(f, "Four-Type (ⲁⲃ)"),
            ScarabType::FiveType => write!(f, "Five-Type (ⲀⲄ)"),
            ScarabType::SixType => write!(f, "Six-Type (ⲄⲄ)"),
            ScarabType::SevenType => write!(f, "Seven-Type (ⲁⲃ)"),
            ScarabType::EightType => write!(f, "Eight-Type (ⲀⲄ)"),
            ScarabType::NineType => write!(f, "Nine-Type (Ⲁⲁ)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_o_type_has_9() {
        assert_eq!(ScarabType::OType.codes().len(), 9);
    }

    #[test]
    fn test_one_type_has_5() {
        assert_eq!(ScarabType::OneType.codes().len(), 5);
    }

    #[test]
    fn test_two_type_has_5() {
        assert_eq!(ScarabType::TwoType.codes().len(), 5);
    }

    #[test]
    fn test_three_type_has_4() {
        assert_eq!(ScarabType::ThreeType.codes().len(), 4);
    }

    #[test]
    fn test_four_type_has_2() {
        assert_eq!(ScarabType::FourType.codes().len(), 2);
    }

    #[test]
    fn test_five_type_has_2() {
        assert_eq!(ScarabType::FiveType.codes().len(), 2);
    }

    #[test]
    fn test_six_type_has_2() {
        assert_eq!(ScarabType::SixType.codes().len(), 2);
    }

    #[test]
    fn test_seven_type_has_2() {
        assert_eq!(ScarabType::SevenType.codes().len(), 2);
    }

    #[test]
    fn test_eight_type_has_2() {
        assert_eq!(ScarabType::EightType.codes().len(), 2);
    }

    #[test]
    fn test_nine_type_has_2() {
        assert_eq!(ScarabType::NineType.codes().len(), 2);
    }

    #[test]
    fn test_total_codes() {
        assert_eq!(ScarabType::OType.count() + ScarabType::OneType.count() + ScarabType::TwoType.count() + ScarabType::ThreeType.count() + ScarabType::FourType.count() + ScarabType::FiveType.count() + ScarabType::SixType.count() + ScarabType::SevenType.count() + ScarabType::EightType.count() + ScarabType::NineType.count(), 27);
    }

    #[test]
    fn test_display_format() {
        let o_type = format!("{}", ScarabType::OType);
        assert!(o_type.contains("O-Type"));
        assert!(o_type.contains("Ⲁ"));
    }
}

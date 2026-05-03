pub const BPB_VICTORY_TARGET: f64 = crate::IGLA_TARGET_BPB;

pub struct HiveAutomaton;

impl HiveAutomaton {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for HiveAutomaton {
    fn default() -> Self {
        Self::new()
    }
}

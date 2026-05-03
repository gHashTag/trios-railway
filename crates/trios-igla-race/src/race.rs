pub struct RaceCoordinator;

impl RaceCoordinator {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for RaceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AshaScheduler;

impl AshaScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AshaScheduler {
    fn default() -> Self {
        Self::new()
    }
}

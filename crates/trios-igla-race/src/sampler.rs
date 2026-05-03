pub struct ConfigSampler;

impl ConfigSampler {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfigSampler {
    fn default() -> Self {
        Self::new()
    }
}

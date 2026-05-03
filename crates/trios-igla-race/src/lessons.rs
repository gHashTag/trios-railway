pub struct LessonStore;

impl LessonStore {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LessonStore {
    fn default() -> Self {
        Self::new()
    }
}

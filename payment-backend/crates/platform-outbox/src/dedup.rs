use std::collections::HashSet;
use uuid::Uuid;

pub struct DedupStore {
    processed: HashSet<Uuid>,
}

impl Default for DedupStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DedupStore {
    pub fn new() -> Self {
        Self { processed: HashSet::new() }
    }

    pub fn is_duplicate(&self, event_id: &Uuid) -> bool {
        self.processed.contains(event_id)
    }

    pub fn mark_processed(&mut self, event_id: Uuid) {
        self.processed.insert(event_id);
    }
}

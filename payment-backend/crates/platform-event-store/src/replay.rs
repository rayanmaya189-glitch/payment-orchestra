use crate::event_store::StoredEvent;

pub struct EventReplayer;

impl EventReplayer {
    pub fn replay<T, F>(events: &[StoredEvent], initial: T, fold: F) -> T
    where
        F: Fn(T, &StoredEvent) -> T,
    {
        events.iter().fold(initial, fold)
    }
}

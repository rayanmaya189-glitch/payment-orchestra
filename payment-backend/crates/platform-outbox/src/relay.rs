use tracing::info;
use std::time::Duration;

pub struct OutboxRelay {
    pub poll_interval: Duration,
}

impl Default for OutboxRelay {
    fn default() -> Self {
        Self { poll_interval: Duration::from_millis(100) }
    }
}

impl OutboxRelay {
    pub async fn run(&self) {
        info!("Outbox relay started (poll interval: {:?})", self.poll_interval);
        loop {
            // TODO: Poll unpublished outbox entries and publish to event bus
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

pub struct EventConsumer {
    pub name: String,
    pub stream_name: String,
    pub last_processed_sequence: Option<i64>,
}

impl EventConsumer {
    pub fn new(name: &str, stream_name: &str) -> Self {
        Self { name: name.to_string(), stream_name: stream_name.to_string(), last_processed_sequence: None }
    }
}

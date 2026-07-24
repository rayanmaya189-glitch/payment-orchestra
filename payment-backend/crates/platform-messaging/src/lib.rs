pub mod event_bus;
pub mod nats_event_bus;
pub mod subject;
pub mod envelope;
pub mod consumer;
pub mod dlq;

// Re-export key types at crate root for accessibility
pub use event_bus::{EventBus, NoopEventBus, ChannelEventBus, EventEnvelope, publish_event_fire_and_forget};

/// Encode a `prost::Message` into protobuf bytes.
///
/// Eliminates the 3-line boilerplate (`let mut buf = Vec::new();` + `prost::Message::encode` + `map_err`)
/// that is repeated across all `encode_event_proto` implementations.
///
/// # Usage
/// ```rust,ignore
/// use platform_messaging::encode_proto;
/// let proto = MyProto { field: "value".into() };
/// let bytes: Vec<u8> = encode_proto!(proto)?;
/// ```
#[macro_export]
macro_rules! encode_proto {
    ($proto:expr) => {{
        let mut __buf = Vec::new();
        prost::Message::encode(&$proto, &mut __buf).map_err(|e| e.to_string())?;
        Ok::<Vec<u8>, String>(__buf)
    }};
}

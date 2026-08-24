/// Placeholder for shared gRPC utilities.
pub fn extract_actor_context(_metadata: &tonic::metadata::MetadataMap) -> Option<String> {
    _metadata.get("x-actor-context").and_then(|v| v.to_str().ok().map(|s| s.to_string()))
}

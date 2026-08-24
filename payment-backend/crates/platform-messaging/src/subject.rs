/// NATS-compatible subject naming helpers.
pub fn build_subject(context: &str, aggregate: &str, event_name: &str, version: u16) -> String {
    format!("events.{}.{}.{}.v{}", context, aggregate, event_name, version)
}

pub fn parse_subject(subject: &str) -> Option<(String, String, String, u16)> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.len() == 5 && parts[0] == "events" {
        let version = parts[4].strip_prefix('v')?.parse().ok()?;
        Some((parts[1].to_string(), parts[2].to_string(), parts[3].to_string(), version))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_subject() {
        let s = build_subject("orchestration", "payment_intent", "payment_authorized", 1);
        assert_eq!(s, "events.orchestration.payment_intent.payment_authorized.v1");
    }

    #[test]
    fn test_parse_subject() {
        let result = parse_subject("events.orchestration.payment_intent.payment_authorized.v1");
        assert!(result.is_some());
        let (ctx, agg, event, ver) = result.unwrap();
        assert_eq!(ctx, "orchestration");
        assert_eq!(ver, 1);
    }
}

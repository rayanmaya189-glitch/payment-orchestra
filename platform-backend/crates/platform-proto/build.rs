fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../proto";

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                format!("{proto_root}/common.proto"),
                format!("{proto_root}/orchestration.proto"),
                format!("{proto_root}/iam.proto"),
                format!("{proto_root}/connector.proto"),
                format!("{proto_root}/compliance.proto"),
                format!("{proto_root}/operator.proto"),
                format!("{proto_root}/reconciliation.proto"),
                format!("{proto_root}/invoice.proto"),
                format!("{proto_root}/subscription.proto"),
                format!("{proto_root}/dispute.proto"),
                format!("{proto_root}/risk.proto"),
                format!("{proto_root}/notification.proto"),
                format!("{proto_root}/analytics.proto"),
                format!("{proto_root}/saga.proto"),
                format!("{proto_root}/ai_assistant.proto"),
                format!("{proto_root}/document.proto"),
            ],
            &[proto_root],
        )?;

    Ok(())
}

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
            ],
            &[proto_root],
        )?;

    Ok(())
}

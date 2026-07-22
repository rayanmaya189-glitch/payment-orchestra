use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../proto/");
    let proto_dir = PathBuf::from("../../proto");
    let protos = &[
        "common.proto",
        "operator.proto",
        "iam.proto",
        "compliance.proto",
        "connector.proto",
        "orchestration.proto",
        "invoice.proto",
        "subscription.proto",
        "payment-link.proto",
        "reconciliation.proto",
        "dispute.proto",
        "risk.proto",
        "ai_assistant.proto",
        "document.proto",
        "notification.proto",
        "analytics.proto",
        "saga.proto",
    ];

    let proto_paths: Vec<PathBuf> = protos.iter()
        .map(|p| proto_dir.join(p))
        .collect();

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        // Map protobuf packages to Rust module paths to avoid super::super references
        .extern_path(".common.v1", "crate::common")
        .compile_protos(&proto_paths, &[&proto_dir])?;

    Ok(())
}

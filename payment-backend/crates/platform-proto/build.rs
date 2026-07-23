fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../proto/");

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let proto_dir = std::path::PathBuf::from("../../proto");

    // ── All proto files ──
    let all_proto_files: Vec<&str> = vec![
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

    let full_paths: Vec<std::path::PathBuf> = all_proto_files
        .iter()
        .map(|f| proto_dir.join(f))
        .collect();
    let proto_strs: Vec<&str> = full_paths.iter().map(|p| p.to_str().unwrap()).collect();

    // ── Step 1: Compile ALL protos with prost-build ──
    // Generates message types. Output files are named after the proto file:
    //   common.proto → common.rs, operator.proto → operator.rs, etc.
    let mut prost_config = prost_build::Config::new();
    // Use package-based naming: package operator.v1 → operator.v1.rs
    prost_config.out_dir(&out_dir);
    prost_config.compile_protos(&proto_strs, &["../../proto"])?;

    // ── Step 2: Rename prost-generated files to tonic-style names ──
    // prost_build generates: {proto_name}.rs (e.g., common.rs, operator.rs)
    // lib.rs expects: {package_name}.rs (e.g., common.v1.rs, operator.v1.rs)
    // Mapping: proto file name → package name (with .v1 suffix)
    for proto_file in &all_proto_files {
        let stem = proto_file.strip_suffix(".proto").unwrap_or(proto_file);
        // For payment-link.proto, the package is payment_link.v1
        let rust_module = stem.replace('-', "_");
        let prost_file = out_dir.join(format!("{}.rs", stem));
        let tonic_file = out_dir.join(format!("{}.v1.rs", rust_module));
        if prost_file.exists() && !tonic_file.exists() {
            std::fs::copy(&prost_file, &tonic_file)?;
        }
    }

    // ── Step 3: Compile ALL service protos with tonic-build for gRPC stubs ──
    // tonic_build wraps prost_build and adds gRPC server/client stubs.
    // It generates files using package-based naming (e.g., operator.v1.rs)
    // which will overwrite our renamed prost files with the gRPC stubs included.
    // All protos with service definitions are listed here.
    let service_proto_files: Vec<&str> = vec![
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
    let service_paths: Vec<std::path::PathBuf> = service_proto_files
        .iter()
        .map(|f| proto_dir.join(f))
        .collect();
    let service_strs: Vec<&str> = service_paths.iter().map(|p| p.to_str().unwrap()).collect();

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .extern_path(".common.v1", "crate::common")
        .compile_protos(&service_strs, &["../../proto"])?;

    Ok(())
}

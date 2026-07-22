fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../proto/");
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    let modules = [
        "common", "operator", "iam", "compliance", "connector",
        "orchestration", "invoice", "subscription", "payment_link",
        "reconciliation", "dispute", "risk", "ai_assistant",
        "document", "notification", "analytics", "saga",
    ];

    for module in &modules {
        let filename = format!("{}.v1.rs", module);
        let path = out_dir.join(&filename);
        if !path.exists() {
            std::fs::write(&path, "")?;
        }
    }

    Ok(())
}

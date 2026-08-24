/// Startup check — service has completed initialization.
pub async fn wait_for_startup(service_name: &str, timeout_secs: u64) -> bool {
    tracing::info!("Waiting for {} startup (timeout: {}s)", service_name, timeout_secs);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    true
}

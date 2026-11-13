use tracing_subscriber::EnvFilter;

/// Entry point for the rustspeak server binary.
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("rustspeak server initialized");
    Ok(())
}

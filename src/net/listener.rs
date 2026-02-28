use crate::config::Config;

pub async fn run_listener(
    config: Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = config.addr();
    Ok(())
}

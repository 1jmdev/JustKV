pub mod config;
pub mod net;

use crate::config::Config;
use crate::net::listener::run_listener;

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::from_env();
    run_listener(config).await
}

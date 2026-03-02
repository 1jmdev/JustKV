pub mod config;
pub mod connection;
pub mod listener;
pub mod profiling;
pub mod pubsub;
pub mod transaction;

#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub async fn run(config: config::Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    listener::run_listener(config).await
}

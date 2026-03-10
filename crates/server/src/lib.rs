#![allow(clippy::result_unit_err)]

pub mod auth;
pub mod backup;
pub mod config;
pub mod connection;
pub mod listener;
pub mod logging;
pub mod persistence;
pub mod profile;

#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub async fn run(config: config::Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::lib::run");
    listener::run_listener(config).await
}

#[cfg(feature = "profiling")]
pub async fn run_with_profile(
    config: config::Config,
    profile_hub: profile::ProfileHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::lib::run_with_profile");
    listener::run_listener_with_profile(config, Some(profile_hub)).await
}

#[cfg(feature = "profiling")]
pub use profiler::{render_result_plain, render_result_pretty};

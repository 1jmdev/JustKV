use crate::config::Config;
use crate::engine::store::Store;
use crate::net::connection::handle_connection;
use crate::net::profiling::{LatencyProfiler, ProfilingConfig};
use crate::net::pubsub::PubSubHub;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

pub async fn run_listener(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(config.addr()).await?;
    let store = Store::new(config.shards);
    let pubsub = PubSubHub::new();
    let profiler = ProfilingConfig::from_env().map(LatencyProfiler::new);

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );
    spawn_cached_clock_updater(store.clone());
    if let Some(profiler) = profiler.as_ref() {
        eprintln!(
            "[latency-profiler] enabled interval={}s slow_threshold={}ms long_threshold={}ms slow_samples={}",
            profiler.report_interval().as_secs(),
            profiler.slow_threshold().as_millis(),
            profiler.long_request_threshold().as_millis(),
            profiler.slow_sample_limit(),
        );
        spawn_latency_reporter(profiler.clone());
    }

    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        let shared_profiler = profiler.clone();
        tokio::spawn(async move {
            let _ = handle_connection(socket, shared_store, shared_pubsub, shared_profiler).await;
        });
    }
}

fn spawn_expiry_sweeper(store: Store, interval: Duration) {
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            let _ = store.sweep_expired();
        }
    });
}

fn spawn_cached_clock_updater(store: Store) {
    tokio::spawn(async move {
        loop {
            store.refresh_cached_time();
            sleep(Duration::from_millis(1)).await;
        }
    });
}

fn spawn_latency_reporter(profiler: std::sync::Arc<LatencyProfiler>) {
    tokio::spawn(async move {
        loop {
            sleep(profiler.report_interval()).await;
            profiler.report_and_reset();
        }
    });
}

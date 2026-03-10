use tokio::task::block_in_place;
use tokio::time::{Duration, sleep};

use engine::store::Store;

pub(crate) fn spawn_expiry_sweeper(store: Store, interval: Duration) {
    let _trace = profiler::scope("server::listener::spawn_expiry_sweeper");
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            block_in_place(|| store.sweep_expired());
        }
    });
}

pub(crate) fn spawn_cached_clock_updater(store: Store) {
    let _trace = profiler::scope("server::listener::spawn_cached_clock_updater");
    tokio::spawn(async move {
        loop {
            store.refresh_cached_time();
            sleep(Duration::from_millis(1)).await;
        }
    });
}

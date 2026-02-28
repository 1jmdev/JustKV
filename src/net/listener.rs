use crate::config::Config;
use crate::engine::store::Store;
use crate::net::connection::handle_connection;
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};

pub async fn run_listener(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(config.addr()).await?;
    let store = Store::new(config.shards);

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );

    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared_store = store.clone();
        tokio::spawn(async move {
            let _ = handle_connection(socket, shared_store).await;
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

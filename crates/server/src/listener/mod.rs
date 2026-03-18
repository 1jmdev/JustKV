mod accept;
mod background;
mod shutdown;

use std::io;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;
use tokio::time::Duration;

use crate::auth::AuthService;
use crate::config::Config;
use crate::connection::ConnectionShared;
use crate::persistence::{self, PersistenceHandle};
use accept::{ProtectedMode, bind_listeners, run_accept_loop};
use background::{spawn_cached_clock_updater, spawn_expiry_sweeper};
use engine::pubsub::PubSubHub;
use engine::store::Store;
use shutdown::shutdown_signal;

pub async fn run_listener(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener_label = config.listener_label();
    let _socket_cleanup = config.socket.as_deref().map(UnixSocketCleanup::new);
    let listeners = bind_listeners(&config).await?;
    let store = Store::new(config.shards);
    let pubsub = PubSubHub::new();
    let auth = AuthService::from_config(&config).map_err(io::Error::other)?;
    let protected_mode = ProtectedMode::new(&config, &auth);
    let restore = persistence::restore(&store, &config).await;
    match restore {
        Ok(restore) => {
            if let Some(stats) = restore.snapshot {
                tracing::info!(
                    keys_loaded = stats.keys_loaded,
                    path = %config.snapshot_path().display(),
                    "loaded snapshot"
                );
            }
            if config.appendonly {
                tracing::info!(
                    commands_replayed = restore.aof_commands_replayed,
                    truncated_tail = restore.aof_tail_truncated,
                    path = %config.appendonly_path().display(),
                    "replayed appendonly file"
                );
            }
        }
        Err(err) => {
            tracing::error!(error = %err, "failed to restore persistence state");
        }
    }
    let persistence = PersistenceHandle::spawn(store.clone(), config.clone());
    let shared = ConnectionShared::new(
        store.clone(),
        pubsub.clone(),
        auth.clone(),
        persistence.clone(),
    );

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );
    spawn_cached_clock_updater(store.clone());
    tracing::info!(
        listener = %listener_label,
        io_threads = config.io_threads,
        sweep_interval_ms = config.sweep_interval_ms,
        save_rules = config.save_rules.len(),
        protected_mode = protected_mode.enabled(),
        appendonly = config.appendonly,
        "listener ready"
    );

    let mut accept_tasks = JoinSet::new();
    for listener in listeners {
        let shared = shared.clone();
        accept_tasks.spawn(async move { run_accept_loop(listener, shared, protected_mode).await });
    }

    loop {
        tokio::select! {
            task = accept_tasks.join_next() => {
                let Some(task_result) = task else {
                    return Ok(());
                };

                match task_result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => return Err(err),
                    Err(err) => return Err(err.into()),
                }
            }
            _ = shutdown_signal() => {
                tracing::warn!("shutdown signal received");
                if let Err(err) = persistence.shutdown() {
                    tracing::error!(error = %err, "persistence shutdown failed");
                }
                return Ok(());
            }
        }
    }
}

pub(crate) type ListenerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

struct UnixSocketCleanup {
    path: PathBuf,
}

impl UnixSocketCleanup {
    fn new(path: &str) -> Self {
        Self {
            path: Path::new(path).to_path_buf(),
        }
    }
}

impl Drop for UnixSocketCleanup {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                tracing::warn!(path = %self.path.display(), error = %err, "failed to remove unix socket");
            }
        }
    }
}

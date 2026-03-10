mod accept;
mod background;
mod shutdown;

use std::io;
use tokio::task::JoinSet;
use tokio::time::Duration;

use crate::auth::AuthService;
use crate::config::Config;
use crate::persistence::{self, PersistenceHandle};
use crate::profile::ProfileHub;
use accept::{bind_reuse_port_listeners, run_accept_loop};
use background::{spawn_cached_clock_updater, spawn_expiry_sweeper};
use engine::pubsub::PubSubHub;
use engine::store::Store;
use shutdown::shutdown_signal;

pub async fn run_listener(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_listener_with_profile(config, None).await
}

pub async fn run_listener_with_profile(
    config: Config,
    profile_hub: Option<ProfileHub>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::listener::run_listener");
    let bind_addr = config.addr();
    let listeners = bind_reuse_port_listeners(bind_addr.clone(), config.io_threads).await?;
    let store = Store::new(config.shards);
    let pubsub = PubSubHub::new();
    let profiler = profile_hub.unwrap_or_else(ProfileHub::disabled);
    let auth = AuthService::from_config(&config).map_err(io::Error::other)?;
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

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );
    spawn_cached_clock_updater(store.clone());
    tracing::info!(
        bind = %bind_addr,
        io_threads = config.io_threads,
        sweep_interval_ms = config.sweep_interval_ms,
        save_rules = config.save_rules.len(),
        appendonly = config.appendonly,
        "listener ready"
    );

    let mut accept_tasks = JoinSet::new();
    for listener in listeners {
        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        let shared_auth = auth.clone();
        let shared_persistence = persistence.clone();
        let shared_profiler = profiler;
        accept_tasks.spawn(async move {
            run_accept_loop(
                listener,
                shared_store,
                shared_pubsub,
                shared_auth,
                shared_persistence,
                shared_profiler,
            )
            .await
        });
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

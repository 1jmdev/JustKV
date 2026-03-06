use crate::auth::AuthService;
use crate::config::Config;
use crate::connection::handle_connection;
use crate::profile::ProfileHub;
use crate::pubsub::PubSubHub;
use crate::{backup, backup::SnapshotStats};
use engine::store::Store;
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::task::block_in_place;
use tokio::time::{Duration, sleep};

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
    let snapshot_path = config.snapshot_path();

    if snapshot_path.exists() {
        match backup::load_snapshot(&store, &snapshot_path).await {
            Ok(stats) => {
                tracing::info!(
                    keys_loaded = stats.keys_loaded,
                    path = %snapshot_path.display(),
                    "loaded snapshot"
                );
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    path = %snapshot_path.display(),
                    "failed to load snapshot"
                );
            }
        }
    }

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );
    spawn_cached_clock_updater(store.clone());
    if config.snapshot_interval_secs > 0 {
        spawn_periodic_snapshot(
            store.clone(),
            snapshot_path.clone(),
            Duration::from_secs(config.snapshot_interval_secs),
        );
    }
    tracing::info!(
        bind = %bind_addr,
        io_threads = config.io_threads,
        sweep_interval_ms = config.sweep_interval_ms,
        snapshot_interval_secs = config.snapshot_interval_secs,
        "listener ready"
    );

    let mut accept_tasks = JoinSet::new();
    for listener in listeners {
        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        let shared_auth = auth.clone();
        let shared_profiler = profiler.clone();
        accept_tasks.spawn(async move {
            run_accept_loop(
                listener,
                shared_store,
                shared_pubsub,
                shared_auth,
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
                if config.snapshot_on_shutdown {
                    let stats = write_snapshot_with_log(store.clone(), snapshot_path.clone(), "shutdown").await;
                    if stats.is_none() {
                        tracing::error!("shutdown snapshot failed");
                    }
                }
                return Ok(());
            }
        }
    }
}

async fn run_accept_loop(
    listener: TcpListener,
    store: Store,
    pubsub: PubSubHub,
    auth: AuthService,
    profiler: ProfileHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _trace = profiler::scope("server::listener::run_accept_loop");
    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        let shared_auth = auth.clone();
        let shared_profiler = profiler.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(
                socket,
                shared_store,
                shared_pubsub,
                shared_auth,
                shared_profiler,
            )
            .await
            {
                tracing::debug!(error = %err, "connection closed with error");
            }
        });
    }
}

async fn bind_reuse_port_listeners(
    bind_addr: String,
    io_threads: usize,
) -> Result<Vec<TcpListener>, io::Error> {
    let _trace = profiler::scope("server::listener::bind_reuse_port_listeners");
    let mut addresses = tokio::net::lookup_host(bind_addr).await?;
    let Some(address) = addresses.next() else {
        return Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no socket address resolved for bind",
        ));
    };

    let listener_count = io_threads.max(1);
    let mut listeners = Vec::with_capacity(listener_count);
    for _ in 0..listener_count {
        listeners.push(bind_single_listener(address)?);
    }

    Ok(listeners)
}

fn bind_single_listener(address: SocketAddr) -> Result<TcpListener, io::Error> {
    let _trace = profiler::scope("server::listener::bind_single_listener");
    let domain = if address.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };

    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&address.into())?;
    socket.listen(2048)?;

    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}

fn spawn_expiry_sweeper(store: Store, interval: Duration) {
    let _trace = profiler::scope("server::listener::spawn_expiry_sweeper");
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            let _ = block_in_place(|| store.sweep_expired());
        }
    });
}

fn spawn_cached_clock_updater(store: Store) {
    let _trace = profiler::scope("server::listener::spawn_cached_clock_updater");
    tokio::spawn(async move {
        loop {
            store.refresh_cached_time();
            sleep(Duration::from_millis(1)).await;
        }
    });
}

fn spawn_periodic_snapshot(store: Store, snapshot_path: PathBuf, interval: Duration) {
    let _trace = profiler::scope("server::listener::spawn_periodic_snapshot");
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            let _ = write_snapshot_with_log(store.clone(), snapshot_path.clone(), "periodic").await;
        }
    });
}

async fn write_snapshot_with_log(
    store: Store,
    snapshot_path: PathBuf,
    reason: &str,
) -> Option<SnapshotStats> {
    let _trace = profiler::scope("server::listener::write_snapshot_with_log");
    let path_for_log = snapshot_path.clone();
    let result =
        tokio::task::spawn_blocking(move || backup::write_snapshot(&store, &snapshot_path)).await;
    match result {
        Ok(Ok(stats)) => {
            tracing::info!(
                reason,
                keys_written = stats.keys_written,
                bytes_written = stats.bytes_written,
                path = %path_for_log.display(),
                "snapshot completed"
            );
            Some(stats)
        }
        Ok(Err(err)) => {
            tracing::error!(
                reason,
                error = %err,
                path = %path_for_log.display(),
                "snapshot failed"
            );
            None
        }
        Err(err) => {
            tracing::error!(
                reason,
                error = %err,
                path = %path_for_log.display(),
                "snapshot task failed"
            );
            None
        }
    }
}

async fn shutdown_signal() {
    let _trace = profiler::scope("server::listener::shutdown_signal");
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {},
                    _ = sigterm.recv() => {},
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

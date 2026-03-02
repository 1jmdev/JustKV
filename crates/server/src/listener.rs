use crate::config::Config;
use crate::connection::handle_connection;
use crate::pubsub::PubSubHub;
use engine::store::Store;
use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::task::block_in_place;
use tokio::time::{Duration, sleep};

pub async fn run_listener(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listeners = bind_reuse_port_listeners(config.addr(), config.io_threads).await?;
    let store = Store::new(config.shards);
    let pubsub = PubSubHub::new();

    spawn_expiry_sweeper(
        store.clone(),
        Duration::from_millis(config.sweep_interval_ms),
    );
    spawn_cached_clock_updater(store.clone());

    let mut accept_tasks = JoinSet::new();
    for listener in listeners {
        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        accept_tasks.spawn(async move {
            run_accept_loop(listener, shared_store, shared_pubsub).await
        });
    }

    loop {
        let Some(task_result) = accept_tasks.join_next().await else {
            return Ok(());
        };

        match task_result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(err.into()),
        }
    }
}

async fn run_accept_loop(
    listener: TcpListener,
    store: Store,
    pubsub: PubSubHub,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        tokio::spawn(async move {
            let _ = handle_connection(socket, shared_store, shared_pubsub).await;
        });
    }
}

async fn bind_reuse_port_listeners(
    bind_addr: String,
    io_threads: usize,
) -> Result<Vec<TcpListener>, io::Error> {
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
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            let _ = block_in_place(|| store.sweep_expired());
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

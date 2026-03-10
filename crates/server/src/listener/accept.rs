use std::io;
use std::net::SocketAddr;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::TcpListener;

use crate::auth::AuthService;
use crate::connection::handle_connection;
use crate::listener::ListenerResult;
use crate::persistence::PersistenceHandle;
use crate::profile::ProfileHub;
use engine::pubsub::PubSubHub;
use engine::store::Store;

pub(crate) async fn run_accept_loop(
    listener: TcpListener,
    store: Store,
    pubsub: PubSubHub,
    auth: AuthService,
    persistence: PersistenceHandle,
    profiler: ProfileHub,
) -> ListenerResult {
    let _trace = profiler::scope("server::listener::run_accept_loop");
    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared_store = store.clone();
        let shared_pubsub = pubsub.clone();
        let shared_auth = auth.clone();
        let shared_persistence = persistence.clone();
        let shared_profiler = profiler;
        tokio::spawn(async move {
            if let Err(err) = handle_connection(
                socket,
                shared_store,
                shared_pubsub,
                shared_auth,
                shared_persistence,
                shared_profiler,
            )
            .await
            {
                tracing::debug!(error = %err, "connection closed with error");
            }
        });
    }
}

pub(crate) async fn bind_reuse_port_listeners(
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

use std::io;
use std::net::SocketAddr;
use std::path::Path;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpListener, UnixListener};

use crate::config::Config;
use crate::connection::{ConnectionShared, ConnectionStream, handle_connection};
use crate::listener::ListenerResult;

pub(crate) enum ServerListener {
    Tcp(TcpListener),
    Unix(UnixListener),
}

pub(crate) async fn run_accept_loop(
    listener: ServerListener,
    shared: ConnectionShared,
) -> ListenerResult {
    match listener {
        ServerListener::Tcp(listener) => run_tcp_accept_loop(listener, shared).await,
        ServerListener::Unix(listener) => run_unix_accept_loop(listener, shared).await,
    }
}

pub(crate) async fn bind_listeners(config: &Config) -> Result<Vec<ServerListener>, io::Error> {
    match config.socket.as_deref() {
        Some(path) => Ok(vec![ServerListener::Unix(bind_unix_listener(path)?)]),
        None => bind_reuse_port_listeners(config.addr(), config.io_threads)
            .await
            .map(|listeners| listeners.into_iter().map(ServerListener::Tcp).collect()),
    }
}

async fn run_tcp_accept_loop(listener: TcpListener, shared: ConnectionShared) -> ListenerResult {
    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let shared = shared.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(ConnectionStream::Tcp(socket), shared).await {
                tracing::debug!(error = %err, "connection closed with error");
            }
        });
    }
}

async fn run_unix_accept_loop(listener: UnixListener, shared: ConnectionShared) -> ListenerResult {
    loop {
        let (socket, _) = listener.accept().await?;

        let shared = shared.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(ConnectionStream::Unix(socket), shared).await {
                tracing::debug!(error = %err, "connection closed with error");
            }
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

fn bind_unix_listener(path: &str) -> Result<UnixListener, io::Error> {
    let socket_path = Path::new(path);
    if let Some(parent) = socket_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::metadata(parent)?;
    }

    match std::fs::symlink_metadata(socket_path) {
        Ok(metadata) => {
            use std::os::unix::fs::FileTypeExt;

            if metadata.file_type().is_socket() {
                std::fs::remove_file(socket_path)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("socket path is not a Unix socket: {path}"),
                ));
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    UnixListener::bind(socket_path)
}

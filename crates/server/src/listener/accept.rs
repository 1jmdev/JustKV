use std::io;
use std::net::SocketAddr;
use std::path::Path;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, UnixListener};

use crate::auth::AuthService;
use crate::config::Config;
use crate::connection::{ConnectionShared, ConnectionStream, handle_connection};
use crate::listener::ListenerResult;

const PROTECTED_MODE_DENIED_RESPONSE: &[u8] = b"-DENIED TCP connections from non-loopback addresses are blocked because protected-mode is enabled and at least one enabled user does not require a password. Configure a password, restrict bind, use a Unix socket, or set protected-mode no.\r\n";

pub(crate) enum ServerListener {
    Tcp(TcpListener),
    Unix(UnixListener),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProtectedMode {
    deny_remote_clients: bool,
}

impl ProtectedMode {
    pub(crate) fn new(config: &Config, auth: &AuthService) -> Self {
        Self {
            deny_remote_clients: config.protected_mode && auth.has_passwordless_user(),
        }
    }

    pub(crate) fn enabled(self) -> bool {
        self.deny_remote_clients
    }

    fn rejects(self, peer_addr: SocketAddr) -> bool {
        self.deny_remote_clients && !peer_addr.ip().is_loopback()
    }
}

pub(crate) async fn run_accept_loop(
    listener: ServerListener,
    shared: ConnectionShared,
    protected_mode: ProtectedMode,
) -> ListenerResult {
    match listener {
        ServerListener::Tcp(listener) => {
            run_tcp_accept_loop(listener, shared, protected_mode).await
        }
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

async fn run_tcp_accept_loop(
    listener: TcpListener,
    shared: ConnectionShared,
    protected_mode: ProtectedMode,
) -> ListenerResult {
    loop {
        let (mut socket, peer_addr) = listener.accept().await?;

        if protected_mode.rejects(peer_addr) {
            tracing::warn!(peer_addr = %peer_addr, "rejected protected-mode tcp client");
            tokio::spawn(async move {
                if let Err(error) = socket.write_all(PROTECTED_MODE_DENIED_RESPONSE).await {
                    tracing::debug!(peer_addr = %peer_addr, error = %error, "failed to write protected-mode denial");
                    return;
                }
                if let Err(error) = socket.shutdown().await {
                    tracing::debug!(peer_addr = %peer_addr, error = %error, "failed to shutdown protected-mode denial socket");
                }
            });
            continue;
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserDirectiveConfig;

    #[test]
    fn protected_mode_rejects_non_loopback_clients_with_passwordless_user() {
        let protected_mode = ProtectedMode::new(
            &Config::default(),
            &AuthService::from_config(&Config::default()).expect("auth service"),
        );

        assert!(protected_mode.rejects(SocketAddr::from(([10, 0, 0, 8], 6379))));
    }

    #[test]
    fn protected_mode_allows_loopback_clients() {
        let protected_mode = ProtectedMode::new(
            &Config::default(),
            &AuthService::from_config(&Config::default()).expect("auth service"),
        );

        assert!(!protected_mode.rejects(SocketAddr::from(([127, 0, 0, 1], 6379))));
    }

    #[test]
    fn protected_mode_allows_remote_clients_when_disabled() {
        let config = Config {
            protected_mode: false,
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");
        let protected_mode = ProtectedMode::new(&config, &auth);

        assert!(!protected_mode.rejects(SocketAddr::from(([10, 0, 0, 8], 6379))));
    }

    #[test]
    fn protected_mode_allows_remote_clients_when_enabled_users_require_passwords() {
        let config = Config {
            user_directives: vec![
                UserDirectiveConfig {
                    name: "default".to_string(),
                    rules: vec!["reset".to_string()],
                },
                UserDirectiveConfig {
                    name: "alice".to_string(),
                    rules: vec![
                        "on".to_string(),
                        ">secret".to_string(),
                        "+@all".to_string(),
                        "allkeys".to_string(),
                        "allchannels".to_string(),
                    ],
                },
            ],
            ..Config::default()
        };
        let auth = AuthService::from_config(&config).expect("auth service");
        let protected_mode = ProtectedMode::new(&config, &auth);

        assert!(!protected_mode.rejects(SocketAddr::from(([10, 0, 0, 8], 6379))));
    }
}

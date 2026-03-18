use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use betterkv_server::auth::{AuthService, SessionAuth, UserDirectiveConfig};
use betterkv_server::backup;
use betterkv_server::config::{Config, SnapshotCompression};
use betterkv_server::connection;
use betterkv_server::persistence::{PersistenceHandle, should_log_command};
use commands::dispatch::CommandId;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine::pubsub::PubSubHub;
use engine::store::Store;
use protocol::types::RespFrame;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use types::value::CompactArg;

static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

const PING_REQUEST: &[u8] = b"*1\r\n$4\r\nPING\r\n";
const PING_RESPONSE: &[u8] = b"+PONG\r\n";
const SET_REQUEST: &[u8] = b"*3\r\n$3\r\nSET\r\n$9\r\nbench:key\r\n$5\r\nvalue\r\n";
const SET_RESPONSE: &[u8] = b"+OK\r\n";
const GET_REQUEST: &[u8] = b"*2\r\n$3\r\nGET\r\n$9\r\nbench:key\r\n";
const GET_RESPONSE: &[u8] = b"$5\r\nvalue\r\n";

fn criterion_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
}

fn arg(value: &str) -> CompactArg {
    CompactArg::from_slice(value.as_bytes())
}

fn persistence_config(data_dir: &Path, appendonly: bool) -> Config {
    Config {
        data_dir: data_dir.display().to_string(),
        dbfilename: "bench.rdb".to_string(),
        appendonly,
        appendfilename: "bench.aof".to_string(),
        snapshot_on_shutdown: false,
        save_rules: Vec::new(),
        ..Config::default()
    }
}

fn connection_config() -> Config {
    Config {
        appendonly: false,
        snapshot_on_shutdown: false,
        save_rules: Vec::new(),
        ..Config::default()
    }
}

fn auth_config() -> Config {
    Config {
        requirepass: Some("secret".to_string()),
        user_directives: vec![UserDirectiveConfig {
            name: "alice".to_string(),
            rules: vec![
                "on".to_string(),
                ">wonderland".to_string(),
                "+GET".to_string(),
                "~cache:*".to_string(),
            ],
        }],
        ..Config::default()
    }
}

fn populated_store(entries: usize) -> Store {
    let store = Store::new(16);
    for index in 0..entries {
        let key = format!("key:{index}");
        let value = format!("value:{index}");
        store.set(key.as_bytes(), value.as_bytes(), None);
    }
    store
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "betterkv-server-bench-{label}-{nanos}-{counter}-{}",
        std::process::id()
    ));
    if let Err(error) = std::fs::create_dir_all(&path) {
        panic!(
            "failed to create temp directory {}: {error}",
            path.display()
        );
    }
    path
}

fn remove_temp_dir(path: &Path) {
    if let Err(error) = std::fs::remove_dir_all(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        panic!(
            "failed to remove temp directory {}: {error}",
            path.display()
        );
    }
}

struct ConnectionHarness {
    client: TcpStream,
    task: tokio::task::JoinHandle<()>,
}

impl ConnectionHarness {
    async fn new(runtime_config: Config) -> Self {
        let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
            Ok(listener) => listener,
            Err(error) => panic!("failed to bind benchmark listener: {error}"),
        };
        let address = match listener.local_addr() {
            Ok(address) => address,
            Err(error) => panic!("failed to inspect benchmark listener address: {error}"),
        };

        let store = Store::new(runtime_config.shards);
        let pubsub = PubSubHub::new();
        let auth = match AuthService::from_config(&runtime_config) {
            Ok(auth) => auth,
            Err(error) => panic!("failed to build auth service: {error}"),
        };
        let persistence = PersistenceHandle::spawn(store.clone(), runtime_config);
        let shared = connection::ConnectionShared::new(store, pubsub, auth, persistence.clone());

        let task = tokio::spawn(async move {
            let accepted = listener.accept().await;
            let (stream, _) = match accepted {
                Ok(pair) => pair,
                Err(error) => panic!("failed to accept benchmark connection: {error}"),
            };

            if let Err(error) = connection::handle_connection(
                betterkv_server::connection::ConnectionStream::Tcp(stream),
                shared,
            )
            .await
            {
                panic!("connection benchmark task failed: {error}");
            }

            if let Err(error) = persistence.shutdown() {
                panic!("failed to shut down persistence handle: {error}");
            }
        });

        let client = match TcpStream::connect(address).await {
            Ok(stream) => stream,
            Err(error) => panic!("failed to connect benchmark client: {error}"),
        };

        Self { client, task }
    }

    async fn round_trip(&mut self, request: &[u8], expected_response: &[u8]) {
        if let Err(error) = self.client.write_all(request).await {
            panic!("failed to send benchmark request: {error}");
        }

        let mut response = vec![0_u8; expected_response.len()];
        if let Err(error) = self.client.read_exact(&mut response).await {
            panic!("failed to read benchmark response: {error}");
        }

        if response != expected_response {
            panic!(
                "unexpected response: expected {:?}, got {:?}",
                expected_response, response
            );
        }
    }

    async fn shutdown(mut self) {
        if let Err(error) = self.client.shutdown().await {
            panic!("failed to close benchmark client: {error}");
        }

        if let Err(error) = self.task.await {
            panic!("benchmark connection task join failed: {error}");
        }
    }
}

fn bench_config(c: &mut Criterion) {
    let config = Config::default();
    let mut group = c.benchmark_group("config");

    group.bench_function("addr", |b| b.iter(|| black_box(config.addr())));
    group.bench_function("snapshot_path", |b| {
        b.iter(|| black_box(config.snapshot_path()))
    });
    group.bench_function("appendonly_path", |b| {
        b.iter(|| black_box(config.appendonly_path()))
    });

    group.finish();
}

fn bench_auth(c: &mut Criterion) {
    let config = auth_config();
    let auth = match AuthService::from_config(&config) {
        Ok(auth) => auth,
        Err(error) => panic!("failed to build auth service: {error}"),
    };
    let dry_run_args = [arg("GET"), arg("cache:1")];
    let denied_args = [arg("GET"), arg("other:1")];
    let mut authorized_session = SessionAuth::clone(&auth.new_session());
    authorized_session.set_user("alice".to_string());
    let mut group = c.benchmark_group("auth");

    group.bench_function("from_config", |b| {
        b.iter(|| black_box(AuthService::from_config(black_box(&config))))
    });
    group.bench_function("new_session", |b| b.iter(|| black_box(auth.new_session())));
    group.bench_function("authenticate_default", |b| {
        b.iter(|| black_box(auth.authenticate(black_box(b"default"), black_box(b"secret"))))
    });
    group.bench_function("authenticate_acl_user", |b| {
        b.iter(|| black_box(auth.authenticate(black_box(b"alice"), black_box(b"wonderland"))))
    });
    group.bench_function("dry_run_get_hit", |b| {
        b.iter(|| {
            black_box(auth.dry_run(
                black_box("alice"),
                black_box(CommandId::Get),
                black_box(&dry_run_args),
            ))
        })
    });
    group.bench_function("dry_run_get_key_miss", |b| {
        b.iter(|| {
            black_box(auth.dry_run(
                black_box("alice"),
                black_box(CommandId::Get),
                black_box(&denied_args),
            ))
        })
    });
    group.bench_function("refresh_session", |b| {
        b.iter(|| {
            let mut session = authorized_session.clone();
            black_box(auth.refresh_session(black_box(&mut session), black_box(auth.acl_epoch())))
        })
    });

    group.finish();
}

fn bench_persistence(c: &mut Criterion) {
    let temp_dir = unique_temp_dir("persistence");
    let config = persistence_config(&temp_dir, true);
    let handle = PersistenceHandle::spawn(Store::new(16), config);
    let ok_response = RespFrame::SimpleStatic("OK");
    let error_response = RespFrame::ErrorStatic("ERR");
    let command_args = [arg("SET"), arg("bench:key"), arg("value")];
    let transaction = vec![
        (
            CommandId::Set,
            vec![arg("SET"), arg("key:1"), arg("value:1")],
        ),
        (
            CommandId::Set,
            vec![arg("SET"), arg("key:2"), arg("value:2")],
        ),
    ];
    let mut group = c.benchmark_group("persistence");

    group.bench_function("should_log_command", |b| {
        b.iter(|| black_box(should_log_command(CommandId::Set, black_box(&ok_response))))
    });
    group.bench_function("should_skip_error_response", |b| {
        b.iter(|| {
            black_box(should_log_command(
                CommandId::Set,
                black_box(&error_response),
            ))
        })
    });
    group.bench_function("record_command_to_buffer", |b| {
        b.iter(|| {
            let mut local_bytes = Vec::with_capacity(128);
            let mut local_dirty = 0_u64;
            handle.record_command_to_buffer(
                black_box(CommandId::Set),
                black_box(&command_args),
                black_box(&ok_response),
                black_box(&mut local_bytes),
                black_box(&mut local_dirty),
            );
            black_box((local_bytes, local_dirty))
        })
    });
    group.bench_function("record_transaction_to_buffer", |b| {
        b.iter(|| {
            let mut local_bytes = Vec::with_capacity(256);
            let mut local_dirty = 0_u64;
            handle.record_transaction_to_buffer(
                black_box(&transaction),
                black_box(&mut local_bytes),
                black_box(&mut local_dirty),
            );
            black_box((local_bytes, local_dirty))
        })
    });

    group.finish();

    if let Err(error) = handle.shutdown() {
        panic!("failed to shut down persistence benchmark handle: {error}");
    }
    remove_temp_dir(&temp_dir);
}

fn bench_snapshot(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("snapshot_io");

    for compression in [SnapshotCompression::None, SnapshotCompression::Lz4] {
        let label = match compression {
            SnapshotCompression::None => "none",
            SnapshotCompression::Lz4 => "lz4",
        };

        group.bench_with_input(
            BenchmarkId::new("write_snapshot", label),
            &compression,
            |b, &compression| {
                b.iter_batched(
                    || {
                        let temp_dir = unique_temp_dir("snapshot-write");
                        let path = temp_dir.join("dump.bkv");
                        (populated_store(1_000), temp_dir, path)
                    },
                    |(store, temp_dir, path)| {
                        let result = backup::write_snapshot(
                            black_box(&store),
                            black_box(&path),
                            compression,
                        );
                        remove_temp_dir(&temp_dir);
                        black_box(result)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("load_snapshot", label),
            &compression,
            |b, &compression| {
                b.iter_batched(
                    || {
                        let temp_dir = unique_temp_dir("snapshot-load");
                        let path = temp_dir.join("dump.bkv");
                        let source_store = populated_store(1_000);
                        if let Err(error) =
                            backup::write_snapshot(&source_store, &path, compression)
                        {
                            panic!("failed to seed snapshot benchmark file: {error}");
                        }
                        (Store::new(16), temp_dir, path)
                    },
                    |(restored_store, temp_dir, path)| {
                        let result = runtime.block_on(backup::load_snapshot(
                            black_box(&restored_store),
                            black_box(&path),
                        ));
                        remove_temp_dir(&temp_dir);
                        black_box(result)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn bench_connection(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("connection_roundtrip");

    group.bench_function("ping", |b| {
        b.iter_custom(|iterations| {
            let mut harness = runtime.block_on(ConnectionHarness::new(connection_config()));
            let start = Instant::now();
            for _ in 0..iterations {
                runtime.block_on(harness.round_trip(PING_REQUEST, PING_RESPONSE));
            }
            let elapsed = start.elapsed();
            runtime.block_on(harness.shutdown());
            elapsed
        })
    });

    group.bench_function("set", |b| {
        b.iter_custom(|iterations| {
            let mut harness = runtime.block_on(ConnectionHarness::new(connection_config()));
            let start = Instant::now();
            for _ in 0..iterations {
                runtime.block_on(harness.round_trip(SET_REQUEST, SET_RESPONSE));
            }
            let elapsed = start.elapsed();
            runtime.block_on(harness.shutdown());
            elapsed
        })
    });

    group.bench_function("get_hit", |b| {
        b.iter_custom(|iterations| {
            let mut harness = runtime.block_on(ConnectionHarness::new(connection_config()));
            runtime.block_on(harness.round_trip(SET_REQUEST, SET_RESPONSE));

            let start = Instant::now();
            for _ in 0..iterations {
                runtime.block_on(harness.round_trip(GET_REQUEST, GET_RESPONSE));
            }
            let elapsed = start.elapsed();
            runtime.block_on(harness.shutdown());
            elapsed
        })
    });

    group.finish();
}

fn build_runtime() -> Runtime {
    match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => panic!("failed to build benchmark runtime: {error}"),
    }
}

criterion_group!(
    name = benches;
    config = criterion_config();
    targets = bench_config, bench_auth, bench_persistence, bench_snapshot, bench_connection
);
criterion_main!(benches);

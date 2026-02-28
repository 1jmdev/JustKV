mod support;

use std::time::Duration;

use justkv::protocol::types::RespFrame;
use support::{connect, send_command, spawn_server};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn incr_and_ttl_commands_work() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let incr_1 = send_command(&mut conn, &[b"INCR", b"counter"]).await;
    assert_eq!(incr_1, RespFrame::Integer(1));

    let incr_2 = send_command(&mut conn, &[b"INCR", b"counter"]).await;
    assert_eq!(incr_2, RespFrame::Integer(2));

    let expire = send_command(&mut conn, &[b"EXPIRE", b"counter", b"1"]).await;
    assert_eq!(expire, RespFrame::Integer(1));

    let ttl = send_command(&mut conn, &[b"TTL", b"counter"]).await;
    match ttl {
        RespFrame::Integer(value) => assert!((0..=1).contains(&value)),
        other => panic!("expected integer ttl response, got {other:?}"),
    }

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let ttl_missing = send_command(&mut conn, &[b"TTL", b"counter"]).await;
    assert_eq!(ttl_missing, RespFrame::Integer(-2));

    server.abort();
}

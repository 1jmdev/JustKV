mod support;

use justkv::protocol::types::{BulkData, RespFrame};
use support::{connect, send_command, spawn_server};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_exec_and_discard_work() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"SET", b"tx:key", b"1"]).await,
        RespFrame::Simple("QUEUED".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"INCR", b"tx:ctr"]).await,
        RespFrame::Simple("QUEUED".to_string())
    );

    assert_eq!(
        send_command(&mut conn, &[b"EXEC"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Simple("OK".to_string()),
            RespFrame::Integer(1),
        ]))
    );

    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"SET", b"tx:key", b"discarded"]).await,
        RespFrame::Simple("QUEUED".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"DISCARD"]).await,
        RespFrame::Simple("OK".to_string())
    );

    assert_eq!(
        send_command(&mut conn, &[b"GET", b"tx:key"]).await,
        RespFrame::Bulk(Some(BulkData::from_vec(b"1".to_vec())))
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watch_and_unwatch_work() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let _ = send_command(&mut conn, &[b"SET", b"watched", b"v1"]).await;

    assert_eq!(
        send_command(&mut conn, &[b"WATCH", b"watched"]).await,
        RespFrame::Simple("OK".to_string())
    );
    let _ = send_command(&mut conn, &[b"SET", b"watched", b"v2"]).await;

    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"SET", b"tx", b"1"]).await,
        RespFrame::Simple("QUEUED".to_string())
    );
    assert_eq!(send_command(&mut conn, &[b"EXEC"]).await, RespFrame::Array(None));

    assert_eq!(
        send_command(&mut conn, &[b"WATCH", b"watched"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"UNWATCH"]).await,
        RespFrame::Simple("OK".to_string())
    );
    let _ = send_command(&mut conn, &[b"SET", b"watched", b"v3"]).await;

    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"SET", b"tx", b"2"]).await,
        RespFrame::Simple("QUEUED".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"EXEC"]).await,
        RespFrame::Array(Some(vec![RespFrame::Simple("OK".to_string())]))
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transaction_errors_match_expected_shape() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    assert_eq!(
        send_command(&mut conn, &[b"EXEC"]).await,
        RespFrame::Error("ERR EXEC without MULTI".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"DISCARD"]).await,
        RespFrame::Error("ERR DISCARD without MULTI".to_string())
    );

    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"MULTI"]).await,
        RespFrame::Error("ERR MULTI calls can not be nested".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"WATCH", b"k"]).await,
        RespFrame::Error("ERR WATCH inside MULTI is not allowed".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"DISCARD"]).await,
        RespFrame::Simple("OK".to_string())
    );

    server.abort();
}

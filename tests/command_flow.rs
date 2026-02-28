mod support;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use support::{connect, send_command, spawn_server};
use valkey::protocol::types::RespFrame;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_get_del_flow_works() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let set = send_command(&mut conn, &[b"SET", b"name", b"maty"]).await;
    assert_eq!(set, RespFrame::Simple("OK".to_string()));

    let get = send_command(&mut conn, &[b"GET", b"name"]).await;
    assert_eq!(get, RespFrame::Bulk(Some(b"maty".to_vec())));

    let del = send_command(&mut conn, &[b"DEL", b"name"]).await;
    assert_eq!(del, RespFrame::Integer(1));

    let get_missing = send_command(&mut conn, &[b"GET", b"name"]).await;
    assert_eq!(get_missing, RespFrame::Bulk(None));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mset_mget_flow_works() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let mset = send_command(&mut conn, &[b"MSET", b"a", b"1", b"b", b"2"]).await;
    assert_eq!(mset, RespFrame::Simple("OK".to_string()));

    let mget = send_command(&mut conn, &[b"MGET", b"a", b"b", b"c"]).await;
    assert_eq!(
        mget,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(b"1".to_vec())),
            RespFrame::Bulk(Some(b"2".to_vec())),
            RespFrame::Bulk(None),
        ]))
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inline_ping_is_supported() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    conn.write_all(b"PING\r\n").await.expect("write inline ping");

    let mut response = [0_u8; 64];
    let read = conn.read(&mut response).await.expect("read inline response");
    assert_eq!(&response[..read], b"+PONG\r\n");

    server.abort();
}

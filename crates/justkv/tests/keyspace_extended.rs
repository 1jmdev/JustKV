mod support;

use justkv::protocol::types::{BulkData, RespFrame};
use support::{connect, send_command, spawn_server};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keyspace_extended_commands_work() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let _ = send_command(&mut conn, &[b"MSET", b"a", b"1", b"b", b"2", b"c", b"3"]).await;

    let scan = send_command(&mut conn, &[b"SCAN", b"0", b"MATCH", b"a*", b"COUNT", b"10"]).await;
    match scan {
        RespFrame::Array(Some(values)) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], RespFrame::Bulk(Some(BulkData::from_vec(b"0".to_vec()))));
            assert_eq!(
                values[1],
                RespFrame::Array(Some(vec![RespFrame::Bulk(Some(BulkData::from_vec(
                    b"a".to_vec(),
                )))],))
            );
        }
        other => panic!("unexpected SCAN response: {other:?}"),
    }

    assert_eq!(
        send_command(&mut conn, &[b"COPY", b"a", b"a2"]).await,
        RespFrame::Integer(1)
    );
    assert_eq!(
        send_command(&mut conn, &[b"COPY", b"a", b"a2"]).await,
        RespFrame::Integer(0)
    );
    assert_eq!(
        send_command(&mut conn, &[b"COPY", b"a", b"a2", b"REPLACE"]).await,
        RespFrame::Integer(1)
    );

    let dumped = send_command(&mut conn, &[b"DUMP", b"a"]).await;
    let payload = match dumped {
        RespFrame::Bulk(Some(BulkData::Arg(value))) => value.to_vec(),
        other => panic!("unexpected DUMP response: {other:?}"),
    };

    assert_eq!(
        send_command(&mut conn, &[b"RESTORE", b"restored", b"0", payload.as_slice()]).await,
        RespFrame::Simple("OK".to_string())
    );
    assert_eq!(
        send_command(&mut conn, &[b"GET", b"restored"]).await,
        RespFrame::Bulk(Some(BulkData::from_vec(b"1".to_vec())))
    );
    assert_eq!(
        send_command(&mut conn, &[b"RESTORE", b"restored", b"0", payload.as_slice()]).await,
        RespFrame::Error("BUSYKEY Target key name already exists.".to_string())
    );

    assert_eq!(
        send_command(&mut conn, &[b"MOVE", b"a", b"0"]).await,
        RespFrame::Integer(0)
    );
    assert_eq!(
        send_command(&mut conn, &[b"MOVE", b"a", b"1"]).await,
        RespFrame::Error("ERR DB index is out of range".to_string())
    );

    assert_eq!(
        send_command(&mut conn, &[b"UNLINK", b"a2", b"restored"]).await,
        RespFrame::Integer(2)
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sort_variants_work() {
    let (server, port) = spawn_server().await;
    let mut conn = connect(port).await;

    let _ = send_command(&mut conn, &[b"RPUSH", b"nums", b"3", b"1", b"2"]).await;

    assert_eq!(
        send_command(&mut conn, &[b"SORT", b"nums"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"1".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"2".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"3".to_vec()))),
        ]))
    );
    assert_eq!(
        send_command(&mut conn, &[b"SORT", b"nums", b"DESC", b"LIMIT", b"0", b"2"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"3".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"2".to_vec()))),
        ]))
    );
    assert_eq!(
        send_command(&mut conn, &[b"SORT", b"nums", b"STORE", b"sorted_nums"]).await,
        RespFrame::Integer(3)
    );
    assert_eq!(
        send_command(&mut conn, &[b"LRANGE", b"sorted_nums", b"0", b"-1"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"1".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"2".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"3".to_vec()))),
        ]))
    );

    server.abort();
}

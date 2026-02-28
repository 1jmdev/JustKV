mod support;

use justkv::protocol::types::{BulkData, RespFrame};
use support::{connect, read_frame, send_command, spawn_server};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_subscribe_and_unsubscribe_work() {
    let (server, port) = spawn_server().await;
    let mut sub = connect(port).await;
    let mut pubc = connect(port).await;

    assert_eq!(
        send_command(&mut sub, &[b"SUBSCRIBE", b"news"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"subscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Integer(1),
        ]))
    );

    assert_eq!(
        send_command(&mut pubc, &[b"PUBLISH", b"news", b"hello"]).await,
        RespFrame::Integer(1)
    );

    assert_eq!(
        read_frame(&mut sub).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"message".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"hello".to_vec()))),
        ]))
    );

    assert_eq!(
        send_command(&mut sub, &[b"UNSUBSCRIBE", b"news"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"unsubscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Integer(0),
        ]))
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pattern_subscribe_and_pubsub_introspection_work() {
    let (server, port) = spawn_server().await;
    let mut psub = connect(port).await;
    let mut pubc = connect(port).await;

    assert_eq!(
        send_command(&mut psub, &[b"PSUBSCRIBE", b"n*"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"psubscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"n*".to_vec()))),
            RespFrame::Integer(1),
        ]))
    );

    assert_eq!(
        send_command(&mut pubc, &[b"SUBSCRIBE", b"news"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"subscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Integer(1),
        ]))
    );

    assert_eq!(
        send_command(&mut psub, &[b"PUBSUB", b"CHANNELS", b"n*"]).await,
        RespFrame::Array(Some(vec![RespFrame::Bulk(Some(BulkData::from_vec(
            b"news".to_vec(),
        )))]))
    );

    assert_eq!(
        send_command(&mut psub, &[b"PUBSUB", b"NUMSUB", b"news", b"other"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Integer(1),
            RespFrame::Bulk(Some(BulkData::from_vec(b"other".to_vec()))),
            RespFrame::Integer(0),
        ]))
    );

    assert_eq!(
        send_command(&mut psub, &[b"PUBSUB", b"NUMPAT"]).await,
        RespFrame::Integer(1)
    );

    assert_eq!(
        send_command(&mut psub, &[b"PUBLISH", b"news", b"hey"]).await,
        RespFrame::Integer(2)
    );

    assert_eq!(
        read_frame(&mut psub).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"pmessage".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"n*".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"hey".to_vec()))),
        ]))
    );

    assert_eq!(
        read_frame(&mut pubc).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"message".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"news".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"hey".to_vec()))),
        ]))
    );

    assert_eq!(
        send_command(&mut psub, &[b"PUNSUBSCRIBE", b"n*"]).await,
        RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(b"punsubscribe".to_vec()))),
            RespFrame::Bulk(Some(BulkData::from_vec(b"n*".to_vec()))),
            RespFrame::Integer(0),
        ]))
    );

    server.abort();
}

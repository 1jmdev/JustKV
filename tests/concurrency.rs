mod support;

use tokio::task::JoinSet;

use support::{connect, send_command, spawn_server};
use valkey::protocol::types::{BulkData, RespFrame};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_increments_are_consistent() {
    let (server, port) = spawn_server().await;

    let mut tasks = JoinSet::new();
    for _ in 0..16 {
        tasks.spawn(async move {
            let mut conn = connect(port).await;
            for _ in 0..40 {
                let _ = send_command(&mut conn, &[b"INCR", b"hot_counter"]).await;
            }
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.expect("task finishes");
    }

    let mut check = connect(port).await;
    let value = send_command(&mut check, &[b"GET", b"hot_counter"]).await;
    assert_eq!(
        value,
        RespFrame::Bulk(Some(BulkData::from_vec(b"640".to_vec())))
    );

    server.abort();
}

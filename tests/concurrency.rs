mod support;

use tokio::task::JoinSet;

use support::{connect, send_command, spawn_server};
use valkey::protocol::types::RespFrame;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_increments_are_consistent() {
    let server = spawn_server().await;

    let mut tasks = JoinSet::new();
    for _ in 0..16 {
        tasks.spawn(async {
            let mut conn = connect().await;
            for _ in 0..40 {
                let _ = send_command(&mut conn, &[b"INCR", b"hot_counter"]).await;
            }
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.expect("task finishes");
    }

    let mut check = connect().await;
    let value = send_command(&mut check, &[b"GET", b"hot_counter"]).await;
    assert_eq!(value, RespFrame::Bulk(Some(b"640".to_vec())));

    server.abort();
}

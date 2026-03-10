use commands::dispatch::{dispatch_with_id, identify};
use commands::transaction::TransactionState;
use engine::store::Store;
use protocol::types::RespFrame;
use types::value::CompactArg;

fn run(state: &mut TransactionState, store: &Store, args: &[&str]) -> RespFrame {
    let mut parsed: Vec<CompactArg> = args
        .iter()
        .map(|value| CompactArg::from_slice(value.as_bytes()))
        .collect();
    let command = identify(parsed[0].as_slice());
    state
        .handle_args_with(store, &mut parsed, command, dispatch_with_id)
        .response
}

fn bulk_bytes(frame: RespFrame) -> Option<Vec<u8>> {
    match frame {
        RespFrame::Bulk(Some(data)) => Some(data.into_vec()),
        _ => None,
    }
}

#[test]
fn multi_queues_and_exec_runs_commands() {
    let store = Store::new(1);
    let mut state = TransactionState::default();

    assert_eq!(run(&mut state, &store, &["MULTI"]), RespFrame::ok());
    assert_eq!(
        run(&mut state, &store, &["SET", "key", "value"]),
        RespFrame::simple_static("QUEUED")
    );
    assert_eq!(
        run(&mut state, &store, &["GET", "key"]),
        RespFrame::simple_static("QUEUED")
    );

    let response = run(&mut state, &store, &["EXEC"]);
    let RespFrame::Array(Some(items)) = response else {
        panic!("expected EXEC array response");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], RespFrame::ok());
    assert_eq!(bulk_bytes(items[1].clone()), Some(b"value".to_vec()));
}

#[test]
fn discard_clears_queued_commands() {
    let store = Store::new(1);
    let mut state = TransactionState::default();

    assert_eq!(run(&mut state, &store, &["MULTI"]), RespFrame::ok());
    assert_eq!(
        run(&mut state, &store, &["SET", "key", "value"]),
        RespFrame::simple_static("QUEUED")
    );
    assert_eq!(run(&mut state, &store, &["DISCARD"]), RespFrame::ok());
    assert_eq!(store.get(b"key").unwrap(), None);
}

#[test]
fn watch_aborts_exec_after_external_change() {
    let store = Store::new(1);
    let mut state = TransactionState::default();

    assert_eq!(run(&mut state, &store, &["WATCH", "key"]), RespFrame::ok());
    assert_eq!(run(&mut state, &store, &["MULTI"]), RespFrame::ok());
    assert_eq!(
        run(&mut state, &store, &["SET", "key", "tx-value"]),
        RespFrame::simple_static("QUEUED")
    );

    assert_eq!(
        dispatch_with_id(
            &store,
            identify(b"SET"),
            &[
                CompactArg::from_slice(b"SET"),
                CompactArg::from_slice(b"key"),
                CompactArg::from_slice(b"outside")
            ]
        ),
        RespFrame::ok()
    );

    assert_eq!(run(&mut state, &store, &["EXEC"]), RespFrame::Array(None));
    assert_eq!(
        store.get(b"key").unwrap(),
        Some(CompactArg::from_slice(b"outside"))
    );
}

#[test]
fn unwatch_removes_watch_before_exec() {
    let store = Store::new(1);
    let mut state = TransactionState::default();

    assert_eq!(run(&mut state, &store, &["WATCH", "key"]), RespFrame::ok());
    assert_eq!(run(&mut state, &store, &["UNWATCH"]), RespFrame::ok());
    assert_eq!(run(&mut state, &store, &["MULTI"]), RespFrame::ok());
    assert_eq!(
        run(&mut state, &store, &["SET", "key", "tx-value"]),
        RespFrame::simple_static("QUEUED")
    );

    assert_eq!(
        dispatch_with_id(
            &store,
            identify(b"SET"),
            &[
                CompactArg::from_slice(b"SET"),
                CompactArg::from_slice(b"key"),
                CompactArg::from_slice(b"outside")
            ]
        ),
        RespFrame::ok()
    );

    assert_eq!(
        run(&mut state, &store, &["EXEC"]),
        RespFrame::Array(Some(vec![RespFrame::ok()]))
    );
    assert_eq!(
        store.get(b"key").unwrap(),
        Some(CompactArg::from_slice(b"tx-value"))
    );
}

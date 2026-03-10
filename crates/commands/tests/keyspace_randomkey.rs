use std::thread;
use std::time::Duration;

use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

fn arg(s: &str) -> CompactArg {
    CompactArg::from_slice(s.as_bytes())
}

#[test]
fn randomkey_returns_nil_for_empty_database() {
    let store = Store::new(1);

    assert_eq!(
        dispatch_args(&store, &[arg("RANDOMKEY")]),
        RespFrame::Bulk(None)
    );
}

#[test]
fn randomkey_returns_the_only_live_key() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("only"), arg("value")]),
        RespFrame::ok()
    );

    assert_eq!(
        dispatch_args(&store, &[arg("RANDOMKEY")]),
        RespFrame::Bulk(Some(BulkData::from_vec(b"only".to_vec())))
    );
}

#[test]
fn randomkey_skips_expired_keys() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(
            &store,
            &[arg("PSETEX"), arg("gone"), arg("1"), arg("value")]
        ),
        RespFrame::ok()
    );

    thread::sleep(Duration::from_millis(5));

    assert_eq!(
        dispatch_args(&store, &[arg("RANDOMKEY")]),
        RespFrame::Bulk(None)
    );
}

#[test]
fn randomkey_returns_one_of_existing_keys() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("alpha"), arg("1")]),
        RespFrame::ok()
    );
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("beta"), arg("2")]),
        RespFrame::ok()
    );

    let response = dispatch_args(&store, &[arg("RANDOMKEY")]);
    let alpha = RespFrame::Bulk(Some(BulkData::from_vec(b"alpha".to_vec())));
    let beta = RespFrame::Bulk(Some(BulkData::from_vec(b"beta".to_vec())));

    assert!(
        response == alpha || response == beta,
        "unexpected response: {response:?}"
    );
}

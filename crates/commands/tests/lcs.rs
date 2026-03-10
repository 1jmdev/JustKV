use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

fn arg(value: &str) -> CompactArg {
    CompactArg::from_slice(value.as_bytes())
}

#[test]
fn lcs_returns_subsequence() {
    let store = Store::new(1);

    assert_eq!(
        dispatch_args(
            &store,
            &[
                arg("MSET"),
                arg("key1"),
                arg("ohmytext"),
                arg("key2"),
                arg("mynewtext")
            ]
        ),
        RespFrame::ok()
    );
    assert_eq!(
        dispatch_args(&store, &[arg("LCS"), arg("key1"), arg("key2")]),
        RespFrame::Bulk(Some(BulkData::from_vec(b"mytext".to_vec())))
    );
}

#[test]
fn lcs_len_and_idx_options_work() {
    let store = Store::new(1);

    assert_eq!(
        dispatch_args(
            &store,
            &[
                arg("MSET"),
                arg("key1"),
                arg("ohmytext"),
                arg("key2"),
                arg("mynewtext")
            ]
        ),
        RespFrame::ok()
    );
    assert_eq!(
        dispatch_args(&store, &[arg("LCS"), arg("key1"), arg("key2"), arg("LEN")]),
        RespFrame::Integer(6)
    );
    assert_eq!(
        dispatch_args(
            &store,
            &[
                arg("LCS"),
                arg("key1"),
                arg("key2"),
                arg("IDX"),
                arg("MINMATCHLEN"),
                arg("4"),
                arg("WITHMATCHLEN"),
            ]
        ),
        RespFrame::Map(vec![
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"matches".to_vec()))),
                RespFrame::Array(Some(vec![RespFrame::Array(Some(vec![
                    RespFrame::Array(Some(vec![RespFrame::Integer(4), RespFrame::Integer(7)])),
                    RespFrame::Array(Some(vec![RespFrame::Integer(5), RespFrame::Integer(8)])),
                    RespFrame::Integer(4),
                ]))]))
            ),
            (
                RespFrame::Bulk(Some(BulkData::from_vec(b"len".to_vec()))),
                RespFrame::Integer(6)
            ),
        ])
    );
}

#[test]
fn lcs_rejects_idx_only_options_without_idx() {
    let store = Store::new(1);

    assert_eq!(
        dispatch_args(
            &store,
            &[arg("LCS"), arg("a"), arg("b"), arg("WITHMATCHLEN")]
        ),
        RespFrame::error_static("ERR syntax error")
    );
}

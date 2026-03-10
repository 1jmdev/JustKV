use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

fn arg(s: &str) -> CompactArg {
    CompactArg::from_slice(s.as_bytes())
}

#[test]
fn substr_returns_inclusive_slice() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("mykey"), arg("This is a string")]),
        RespFrame::ok()
    );

    assert_eq!(
        dispatch_args(&store, &[arg("SUBSTR"), arg("mykey"), arg("0"), arg("3")]),
        RespFrame::Bulk(Some(BulkData::from_vec(b"This".to_vec())))
    );
}

#[test]
fn substr_supports_negative_and_out_of_range_offsets() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("mykey"), arg("This is a string")]),
        RespFrame::ok()
    );

    assert_eq!(
        dispatch_args(&store, &[arg("SUBSTR"), arg("mykey"), arg("-3"), arg("-1")]),
        RespFrame::Bulk(Some(BulkData::from_vec(b"ing".to_vec())))
    );
    assert_eq!(
        dispatch_args(
            &store,
            &[arg("SUBSTR"), arg("mykey"), arg("10"), arg("100")]
        ),
        RespFrame::Bulk(Some(BulkData::from_vec(b"string".to_vec())))
    );
}

#[test]
fn substr_matches_getrange() {
    let store = Store::new(1);
    assert_eq!(
        dispatch_args(&store, &[arg("SET"), arg("mykey"), arg("This is a string")]),
        RespFrame::ok()
    );

    let substr = dispatch_args(&store, &[arg("SUBSTR"), arg("mykey"), arg("0"), arg("-1")]);
    let getrange = dispatch_args(
        &store,
        &[arg("GETRANGE"), arg("mykey"), arg("0"), arg("-1")],
    );

    assert_eq!(substr, getrange);
}

use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::{CompactArg, StreamId};

fn arg(value: &str) -> CompactArg {
    CompactArg::from_slice(value.as_bytes())
}

fn run(store: &Store, args: &[&str]) -> RespFrame {
    let parsed: Vec<CompactArg> = args.iter().map(|value| arg(value)).collect();
    dispatch_args(store, &parsed)
}

fn bulk_bytes(frame: RespFrame) -> Vec<u8> {
    match frame {
        RespFrame::Bulk(Some(BulkData::Arg(value))) => value.to_vec(),
        RespFrame::Bulk(Some(BulkData::Value(value))) => value.to_vec(),
        other => panic!("expected bulk frame, got {other:?}"),
    }
}

fn parse_stream_id(frame: RespFrame) -> StreamId {
    let raw = bulk_bytes(frame);
    let text = match String::from_utf8(raw) {
        Ok(value) => value,
        Err(error) => panic!("stream id should be utf8: {error}"),
    };
    let Some((ms, seq)) = text.split_once('-') else {
        panic!("stream id should contain dash: {text}");
    };
    let ms = match ms.parse::<u64>() {
        Ok(value) => value,
        Err(error) => panic!("stream ms should parse: {error}"),
    };
    let seq = match seq.parse::<u64>() {
        Ok(value) => value,
        Err(error) => panic!("stream seq should parse: {error}"),
    };
    StreamId { ms, seq }
}

#[test]
fn delex_supports_value_and_digest_conditions() {
    let store = Store::new(1);

    assert_eq!(run(&store, &["SET", "key", "value"]), RespFrame::ok());
    assert_eq!(
        run(&store, &["DELEX", "key", "IFEQ", "other"]),
        RespFrame::Integer(0)
    );
    assert_eq!(
        run(&store, &["DELEX", "key", "IFNE", "other"]),
        RespFrame::Integer(1)
    );

    assert_eq!(run(&store, &["SET", "key", "value"]), RespFrame::ok());
    let digest = match store.digest(b"key") {
        Ok(Some(value)) => value,
        Ok(None) => panic!("digest should exist"),
        Err(()) => panic!("digest should not fail for string"),
    };
    let digest = match String::from_utf8(digest) {
        Ok(value) => value,
        Err(error) => panic!("digest should be utf8: {error}"),
    };
    assert_eq!(
        run(&store, &["DELEX", "key", "IFDEQ", digest.as_str()]),
        RespFrame::Integer(1)
    );
}

#[test]
fn delex_with_condition_rejects_non_string_keys() {
    let store = Store::new(1);

    assert_eq!(
        run(&store, &["LPUSH", "list", "value"]),
        RespFrame::Integer(1)
    );
    assert_eq!(
        run(&store, &["DELEX", "list", "IFEQ", "value"]),
        RespFrame::error_static(
            "WRONGTYPE Operation against a key holding the wrong kind of value"
        )
    );
    assert_eq!(run(&store, &["DELEX", "list"]), RespFrame::Integer(1));
}

#[test]
fn xdelex_keepref_and_delref_handle_pending_references() {
    let store = Store::new(1);

    let id = parse_stream_id(run(&store, &["XADD", "stream", "*", "field", "value"]));
    assert_eq!(
        run(&store, &["XGROUP", "CREATE", "stream", "group", "0-0"]),
        RespFrame::ok()
    );
    let id_string = format!("{}-{}", id.ms, id.seq);
    let _ = run(
        &store,
        &[
            "XREADGROUP",
            "GROUP",
            "group",
            "consumer",
            "COUNT",
            "1",
            "STREAMS",
            "stream",
            ">",
        ],
    );

    assert_eq!(
        run(
            &store,
            &["XDELEX", "stream", "IDS", "1", id_string.as_str()]
        ),
        RespFrame::Array(Some(vec![RespFrame::Integer(1)]))
    );
    let pending = match store.xpending_summary(b"stream", b"group") {
        Ok(Some(value)) => value,
        Ok(None) => panic!("pending summary should exist"),
        Err(()) => panic!("xpending summary should not fail"),
    };
    assert_eq!(pending.total, 1);

    assert_eq!(
        run(
            &store,
            &["XDELEX", "stream", "DELREF", "IDS", "1", id_string.as_str()]
        ),
        RespFrame::Array(Some(vec![RespFrame::Integer(1)]))
    );
    let pending = match store.xpending_summary(b"stream", b"group") {
        Ok(Some(value)) => value,
        Ok(None) => panic!("pending summary should still exist"),
        Err(()) => panic!("xpending summary should not fail"),
    };
    assert_eq!(pending.total, 0);
}

#[test]
fn xdelex_acked_requires_all_pending_references_to_be_cleared() {
    let store = Store::new(1);

    let id = parse_stream_id(run(&store, &["XADD", "stream", "*", "field", "value"]));
    assert_eq!(
        run(&store, &["XGROUP", "CREATE", "stream", "group", "0-0"]),
        RespFrame::ok()
    );
    let _ = run(
        &store,
        &[
            "XREADGROUP",
            "GROUP",
            "group",
            "consumer",
            "COUNT",
            "1",
            "STREAMS",
            "stream",
            ">",
        ],
    );

    let id_string = format!("{}-{}", id.ms, id.seq);
    assert_eq!(
        run(
            &store,
            &["XDELEX", "stream", "ACKED", "IDS", "1", id_string.as_str()]
        ),
        RespFrame::Array(Some(vec![RespFrame::Integer(2)]))
    );
    assert_eq!(
        run(&store, &["XACK", "stream", "group", id_string.as_str()]),
        RespFrame::Integer(1)
    );
    assert_eq!(
        run(
            &store,
            &["XDELEX", "stream", "ACKED", "IDS", "1", id_string.as_str()]
        ),
        RespFrame::Array(Some(vec![RespFrame::Integer(1)]))
    );
}

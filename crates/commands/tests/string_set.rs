use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::RespFrame;
use types::value::CompactArg;

fn arg(s: &str) -> CompactArg {
    CompactArg::from_slice(s.as_bytes())
}

#[test]
fn set_accepts_mixed_case_expiry_options() {
    let store = Store::new(1);
    let args = [arg("SET"), arg("key"), arg("value"), arg("pX"), arg("1000")];

    assert_eq!(dispatch_args(&store, &args), RespFrame::ok());
    assert_eq!(store.get(b"key").unwrap(), Some(arg("value")));
    assert!(store.pttl(b"key") > 0);
}

#[test]
fn set_accepts_mixed_case_conditional_and_get_options() {
    let store = Store::new(1);

    let seed = [arg("SET"), arg("key"), arg("first")];
    assert_eq!(dispatch_args(&store, &seed), RespFrame::ok());

    let args = [arg("SET"), arg("key"), arg("second"), arg("gEt"), arg("xX")];
    assert_eq!(
        dispatch_args(&store, &args),
        RespFrame::PreEncoded(bytes::Bytes::from_static(b"$5\r\nfirst\r\n"))
    );
    assert_eq!(store.get(b"key").unwrap(), Some(arg("second")));
}

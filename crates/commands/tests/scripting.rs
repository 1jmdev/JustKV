use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use commands::dispatch::dispatch_args;
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn run(store: &Store, args: &[&str]) -> RespFrame {
    let parsed: Vec<CompactArg> = args
        .iter()
        .map(|value| CompactArg::from_slice(value.as_bytes()))
        .collect();
    dispatch_args(store, &parsed)
}

fn bulk_bytes(frame: RespFrame) -> Option<Vec<u8>> {
    match frame {
        RespFrame::Bulk(Some(BulkData::Arg(value))) => Some(value.to_vec()),
        RespFrame::Bulk(Some(BulkData::Value(value))) => Some(value.to_vec()),
        RespFrame::PreEncoded(value) => decode_preencoded_bulk(value.as_ref()),
        _ => None,
    }
}

fn decode_preencoded_bulk(value: &[u8]) -> Option<Vec<u8>> {
    if value == b"$-1\r\n" {
        return None;
    }
    if value.first() != Some(&b'$') {
        return None;
    }
    let len_end = value.windows(2).position(|window| window == b"\r\n")?;
    let len = std::str::from_utf8(&value[1..len_end])
        .ok()?
        .parse::<usize>()
        .ok()?;
    let start = len_end + 2;
    let end = start + len;
    if value.len() != end + 2 || &value[end..] != b"\r\n" {
        return None;
    }
    Some(value[start..end].to_vec())
}

#[test]
fn eval_set_then_return_string() {
    let _guard = test_lock().lock().expect("test lock poisoned");
    let store = Store::new(1);

    let response = run(
        &store,
        &[
            "EVAL",
            "redis.call('SET', KEYS[1], ARGV[1]); return 'Success! Set ' .. KEYS[1] .. ' to ' .. ARGV[1]",
            "1",
            "user:100",
            "John Doe",
        ],
    );

    let payload = bulk_bytes(response).expect("expected bulk response from EVAL");
    assert_eq!(
        payload,
        b"Success! Set user:100 to John Doe".to_vec(),
        "script returned unexpected string"
    );

    let get_response = run(&store, &["GET", "user:100"]);
    let get_payload = bulk_bytes(get_response).expect("expected GET to return bulk value");
    assert_eq!(get_payload, b"John Doe".to_vec());
}

#[test]
fn evalsha_ro_rejects_write_calls() {
    let _guard = test_lock().lock().expect("test lock poisoned");
    let store = Store::new(1);

    let digest = bulk_bytes(run(
        &store,
        &[
            "SCRIPT",
            "LOAD",
            "return redis.call('SET', KEYS[1], ARGV[1])",
        ],
    ))
    .expect("expected SCRIPT LOAD to return sha1 digest");

    let digest = String::from_utf8(digest).expect("digest should be valid utf8");
    let response = run(&store, &["EVALSHA_RO", &digest, "1", "k", "v"]);

    match response {
        RespFrame::Error(message) => {
            assert!(
                message.contains("Write commands are not allowed from read-only scripts"),
                "unexpected error: {message}"
            );
        }
        other => panic!("expected error for EVALSHA_RO write, got: {other:?}"),
    }
}

#[test]
fn script_kill_stops_running_script() {
    let _guard = test_lock().lock().expect("test lock poisoned");
    let store = Store::new(1);
    let worker_store = store.clone();

    let (tx, rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let response = run(&worker_store, &["EVAL", "while true do end", "0"]);
        let _ = tx.send(response);
    });

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut kill_sent = false;
    while Instant::now() < deadline {
        match run(&store, &["SCRIPT", "KILL"]) {
            RespFrame::SimpleStatic("OK") => {
                kill_sent = true;
                break;
            }
            RespFrame::Simple(value) if value == "OK" => {
                kill_sent = true;
                break;
            }
            RespFrame::Error(message)
                if message == "NOTBUSY No scripts in execution right now." =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            other => panic!("unexpected SCRIPT KILL response: {other:?}"),
        }
    }

    assert!(kill_sent, "SCRIPT KILL never observed a running script");

    let response = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("killed script did not finish in time");
    match response {
        RespFrame::Error(message) => {
            assert!(message.contains("Script killed by user with SCRIPT KILL"));
        }
        other => panic!("expected script to end with error after kill, got: {other:?}"),
    }

    handle.join().expect("script worker thread panicked");
}

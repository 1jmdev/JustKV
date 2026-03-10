use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

use crate::util::{Args, eq_ascii, wrong_args};

const OBJECT_FREQ_DISABLED: &str =
    "ERR An LFU maxmemory policy is not selected, access frequency not tracked.";

pub(crate) fn object(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::object::object");
    if args.len() != 3 {
        return wrong_args("OBJECT");
    }

    let subcommand = args[1].as_slice();
    let key = args[2].as_slice();

    if eq_ascii(subcommand, b"ENCODING") {
        return RespFrame::Bulk(
            store
                .object_encoding(key)
                .map(|value| BulkData::from_vec(value.as_bytes().to_vec())),
        );
    }

    if eq_ascii(subcommand, b"FREQ") {
        return match store.object_freq(key) {
            Ok(Some(value)) => RespFrame::Integer(value),
            Ok(None) => RespFrame::Bulk(None),
            Err(()) => RespFrame::error_static(OBJECT_FREQ_DISABLED),
        };
    }

    if eq_ascii(subcommand, b"IDLETIME") {
        return match store.object_idletime(key) {
            Some(value) => RespFrame::Integer(value),
            None => RespFrame::Bulk(None),
        };
    }

    if eq_ascii(subcommand, b"REFCOUNT") {
        return match store.object_refcount(key) {
            Some(value) => RespFrame::Integer(value),
            None => RespFrame::Bulk(None),
        };
    }

    RespFrame::error_static("ERR syntax error")
}

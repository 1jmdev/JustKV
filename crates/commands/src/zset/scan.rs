use crate::util::{Args, f64_to_bytes, parse_scan_options, scan_array_response, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn zscan(store: &Store, args: &Args) -> RespFrame {
    let options = match parse_scan_options(args, "ZSCAN") {
        Ok(options) => options,
        Err(response) => return response,
    };

    match store.zscan(&args[1], options.cursor, options.pattern, options.count) {
        Ok((next, items)) => {
            let mut payload = Vec::with_capacity(items.len().saturating_mul(2));
            for (member, score) in items {
                payload.push(RespFrame::Bulk(Some(BulkData::Arg(member))));
                payload.push(RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(
                    score,
                )))));
            }
            scan_array_response(next, payload)
        }
        Err(_) => wrong_type(),
    }
}

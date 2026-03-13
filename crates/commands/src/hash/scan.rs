use crate::util::{Args, parse_scan_options, scan_array_response, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn hscan(store: &Store, args: &Args) -> RespFrame {
    let options = match parse_scan_options(args, "HSCAN") {
        Ok(options) => options,
        Err(response) => return response,
    };

    match store.hscan(&args[1], options.cursor, options.pattern, options.count) {
        Ok((next_cursor, pairs)) => {
            let mut items = Vec::with_capacity(pairs.len().saturating_mul(2));
            for (field, value) in pairs {
                items.push(RespFrame::Bulk(Some(BulkData::Arg(field))));
                items.push(RespFrame::Bulk(Some(BulkData::Value(value))));
            }
            scan_array_response(next_cursor, items)
        }
        Err(_) => wrong_type(),
    }
}

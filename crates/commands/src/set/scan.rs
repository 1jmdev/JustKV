use crate::util::{Args, parse_scan_options, scan_array_response, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn sscan(store: &Store, args: &Args) -> RespFrame {
    let options = match parse_scan_options(args, "SSCAN") {
        Ok(options) => options,
        Err(response) => return response,
    };

    match store.sscan(&args[1], options.cursor, options.pattern, options.count) {
        Ok((next, members)) => scan_array_response(
            next,
            members
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        ),
        Err(_) => wrong_type(),
    }
}

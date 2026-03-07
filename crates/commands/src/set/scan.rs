use crate::util::{
    eq_ascii, int_error, parse_u64_bytes, u64_to_bytes, wrong_args, wrong_type, Args,
};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn sscan(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::scan::sscan");
    if args.len() < 3 {
        return wrong_args("SSCAN");
    }
    let cursor = match parse_u64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut pattern = None;
    let mut count = 10usize;
    let mut index = 3;
    while index < args.len() {
        if eq_ascii(&args[index], b"MATCH") {
            index += 1;
            if index >= args.len() {
                return crate::util::syntax_error();
            }
            pattern = Some(args[index].as_slice());
        } else if eq_ascii(&args[index], b"COUNT") {
            index += 1;
            if index >= args.len() {
                return crate::util::syntax_error();
            }
            count = match parse_usize(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
        } else {
            return crate::util::syntax_error();
        }
        index += 1;
    }

    match store.sscan(&args[1], cursor, pattern, count) {
        Ok((next, members)) => RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(u64_to_bytes(next)))),
            RespFrame::Array(Some(
                members
                    .into_iter()
                    .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                    .collect(),
            )),
        ])),
        Err(_) => wrong_type(),
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    parse_u64_bytes(raw).ok_or_else(int_error)
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("commands::set::scan::parse_usize");
    let value = parse_u64(raw)?;
    usize::try_from(value).map_err(|_| int_error())
}

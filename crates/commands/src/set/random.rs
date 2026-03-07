use crate::util::{int_error, parse_i64_bytes, parse_u64_bytes, wrong_args, wrong_type, Args};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn spop(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::random::spop");
    if args.len() != 2 && args.len() != 3 {
        return wrong_args("SPOP");
    }
    let has_count = args.len() == 3;
    let count = if has_count {
        match parse_usize(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        1
    };

    match store.spop(&args[1], count) {
        Ok(Some(values)) if has_count => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        )),
        Ok(Some(mut values)) => RespFrame::Bulk(values.pop().map(BulkData::Arg)),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn srandmember(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::set::random::srandmember");
    if args.len() != 2 && args.len() != 3 {
        return wrong_args("SRANDMEMBER");
    }
    let count = if args.len() == 3 {
        match parse_i64(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        1
    };

    match store.srandmember(&args[1], count) {
        Ok(Some(values)) if args.len() == 3 => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|member| RespFrame::Bulk(Some(BulkData::Arg(member))))
                .collect(),
        )),
        Ok(Some(mut values)) => RespFrame::Bulk(values.pop().map(BulkData::Arg)),
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

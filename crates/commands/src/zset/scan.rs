use crate::util::{Args, eq_ascii, f64_to_bytes, int_error, u64_to_bytes, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn zscan(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::zset::scan::zscan");
    if args.len() < 3 {
        return wrong_args("ZSCAN");
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
                return RespFrame::Error("ERR syntax error".to_string());
            }
            pattern = Some(args[index].as_slice());
        } else if eq_ascii(&args[index], b"COUNT") {
            index += 1;
            if index >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            count = match parse_usize(&args[index]) {
                Ok(value) => value,
                Err(response) => return response,
            };
        } else {
            return RespFrame::Error("ERR syntax error".to_string());
        }
        index += 1;
    }

    match store.zscan(&args[1], cursor, pattern, count) {
        Ok((next, items)) => {
            let mut payload = Vec::with_capacity(items.len() * 2);
            for (member, score) in items {
                payload.push(RespFrame::Bulk(Some(BulkData::Arg(member))));
                payload.push(RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(
                    score,
                )))));
            }
            RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::from_vec(u64_to_bytes(next)))),
                RespFrame::Array(Some(payload)),
            ]))
        }
        Err(_) => wrong_type(),
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::zset::scan::parse_u64");
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let _trace = profiler::scope("crates::commands::src::zset::scan::parse_usize");
    let value = parse_u64(raw)?;
    usize::try_from(value).map_err(|_| int_error())
}

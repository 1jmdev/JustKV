use crate::commands::util::{eq_ascii, int_error, u64_to_bytes, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn hscan(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 3 {
        return wrong_args("HSCAN");
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

    match store.hscan(&args[1], cursor, pattern, count) {
        Ok((next_cursor, pairs)) => {
            let mut items = Vec::with_capacity(pairs.len() * 2);
            for (field, value) in pairs {
                items.push(RespFrame::Bulk(Some(BulkData::Arg(field))));
                items.push(RespFrame::Bulk(Some(BulkData::Value(value))));
            }

            RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::from_vec(u64_to_bytes(next_cursor)))),
                RespFrame::Array(Some(items)),
            ]))
        }
        Err(_) => wrong_type(),
    }
}

fn parse_u64(raw: &[u8]) -> Result<u64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_u64(raw)?;
    usize::try_from(value).map_err(|_| int_error())
}

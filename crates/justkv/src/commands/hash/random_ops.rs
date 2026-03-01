use crate::commands::util::{eq_ascii, int_error, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn hrandfield(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 2 || args.len() > 4 {
        return wrong_args("HRANDFIELD");
    }

    if args.len() == 2 {
        return match store.hrandfield_one(&args[1]) {
            Ok(field) => RespFrame::Bulk(field.map(BulkData::Arg)),
            Err(_) => wrong_type(),
        };
    }

    let count = match parse_i64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let with_values = args.len() == 4;
    if with_values && !eq_ascii(&args[3], b"WITHVALUES") {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    match store.hrandfield_pairs(&args[1], count) {
        Ok(pairs) => {
            if with_values {
                let mut items = Vec::with_capacity(pairs.len() * 2);
                for (field, value) in pairs {
                    items.push(RespFrame::Bulk(Some(BulkData::Arg(field))));
                    items.push(RespFrame::Bulk(Some(BulkData::Value(value))));
                }
                RespFrame::Array(Some(items))
            } else {
                RespFrame::Array(Some(
                    pairs
                        .into_iter()
                        .map(|(field, _)| RespFrame::Bulk(Some(BulkData::Arg(field))))
                        .collect(),
                ))
            }
        }
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

use crate::commands::util::{eq_ascii, f64_to_bytes, int_error, wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn zop(store: &Store, args: &Args, command: &str) -> RespFrame {
    if args.len() < 3 {
        return wrong_args(command);
    }
    let num_keys = match parse_usize(&args[1]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if num_keys == 0 {
        return RespFrame::Error("ERR numkeys should be greater than 0".to_string());
    }
    if args.len() < 2 + num_keys {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    let keys_end = 2 + num_keys;
    let withscores = if args.len() == keys_end {
        false
    } else if args.len() == keys_end + 1 && eq_ascii(&args[keys_end], b"WITHSCORES") {
        true
    } else {
        return RespFrame::Error("ERR syntax error".to_string());
    };

    let result = match command {
        "ZINTER" => store.zinter(&args[2..keys_end]),
        "ZUNION" => store.zunion(&args[2..keys_end]),
        "ZDIFF" => store.zdiff(&args[2..keys_end]),
        _ => unreachable!(),
    };

    match result {
        Ok(items) => {
            if withscores {
                RespFrame::Array(Some(
                    items
                        .into_iter()
                        .flat_map(|(member, score)| {
                            [
                                RespFrame::Bulk(Some(BulkData::Arg(member))),
                                RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(score)))),
                            ]
                        })
                        .collect(),
                ))
            } else {
                RespFrame::Array(Some(
                    items
                        .into_iter()
                        .map(|(member, _)| RespFrame::Bulk(Some(BulkData::Arg(member))))
                        .collect(),
                ))
            }
        }
        Err(_) => wrong_type(),
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

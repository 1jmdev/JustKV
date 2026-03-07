use crate::util::{
    eq_ascii, f64_to_bytes, int_error, parse_i64_bytes, wrong_args, wrong_type, Args,
};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn zrandmember(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::zset::random::zrandmember");
    if args.len() < 2 || args.len() > 4 {
        return wrong_args("ZRANDMEMBER");
    }

    let mut withscores = false;
    let count = if args.len() >= 3 {
        let parsed = match parse_i64(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        if args.len() == 4 {
            if !eq_ascii(&args[3], b"WITHSCORES") {
                return crate::util::syntax_error();
            }
            withscores = true;
        }
        parsed
    } else {
        1
    };

    match store.zrandmember(&args[1], count) {
        Ok(Some(items)) if args.len() >= 3 => {
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
        Ok(Some(mut items)) => {
            RespFrame::Bulk(items.pop().map(|(member, _)| BulkData::Arg(member)))
        }
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    parse_i64_bytes(raw).ok_or_else(int_error)
}

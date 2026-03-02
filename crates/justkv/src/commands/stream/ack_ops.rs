use crate::commands::stream::parse::{parse_stream_id, stream_id_to_bulk};
use crate::commands::util::{wrong_args, wrong_type, Args};
use crate::engine::store::Store;
use crate::protocol::types::{BulkData, RespFrame};

pub(crate) fn xack(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 4 {
        return wrong_args("XACK");
    }

    let mut ids = Vec::with_capacity(args.len() - 3);
    for raw in &args[3..] {
        match parse_stream_id(raw) {
            Ok(value) => ids.push(value),
            Err(response) => return response,
        }
    }

    match store.xack(&args[1], &args[2], &ids) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn xpending(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("XPENDING");
    }

    match store.xpending_summary(&args[1], &args[2]) {
        Ok(Some(summary)) => {
            let consumers = RespFrame::Array(Some(
                summary
                    .consumers
                    .into_iter()
                    .map(|(consumer, count)| {
                        RespFrame::Array(Some(vec![
                            RespFrame::Bulk(Some(BulkData::Arg(consumer))),
                            RespFrame::Integer(count),
                        ]))
                    })
                    .collect(),
            ));
            RespFrame::Array(Some(vec![
                RespFrame::Integer(summary.total),
                summary.min.map_or(RespFrame::Bulk(None), stream_id_to_bulk),
                summary.max.map_or(RespFrame::Bulk(None), stream_id_to_bulk),
                consumers,
            ]))
        }
        Ok(None) => RespFrame::Array(Some(vec![
            RespFrame::Integer(0),
            RespFrame::Bulk(None),
            RespFrame::Bulk(None),
            RespFrame::Array(Some(vec![])),
        ])),
        Err(_) => wrong_type(),
    }
}

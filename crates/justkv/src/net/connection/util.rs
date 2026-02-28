use crate::protocol::types::{BulkData, RespFrame};

pub(super) fn parse_args(frame: &RespFrame) -> Result<Vec<Vec<u8>>, String> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error".to_string());
    };

    let mut args = Vec::with_capacity(items.len());
    for item in items {
        match item {
            RespFrame::Bulk(Some(BulkData::Arg(bytes))) => args.push(bytes.to_vec()),
            RespFrame::Bulk(Some(BulkData::Value(bytes))) => args.push(bytes.to_vec()),
            RespFrame::Simple(value) => args.push(value.as_bytes().to_vec()),
            _ => return Err("ERR invalid argument type".to_string()),
        }
    }

    Ok(args)
}

pub(super) fn wrong_args(command: &str) -> RespFrame {
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

pub(super) fn collapse_pubsub_responses(mut responses: Vec<RespFrame>) -> RespFrame {
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespFrame::Array(Some(responses))
    }
}

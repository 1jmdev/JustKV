use crate::commands::{connection, keyspace, string, ttl, util};
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn dispatch(store: &Store, frame: RespFrame) -> RespFrame {
    let args = match parse_command(frame) {
        Ok(args) => args,
        Err(err) => return RespFrame::Error(err),
    };

    if args.is_empty() {
        return RespFrame::Error("ERR empty command".to_string());
    }

    let command = util::upper(&args[0]);
    if let Some(response) = connection::handle(&command, &args) {
        return response;
    }
    if let Some(response) = string::handle(store, &command, &args) {
        return response;
    }
    if let Some(response) = keyspace::handle(store, &command, &args) {
        return response;
    }
    if let Some(response) = ttl::handle(store, &command, &args) {
        return response;
    }

    RespFrame::Error("ERR unknown command".to_string())
}

fn parse_command(frame: RespFrame) -> Result<Vec<Vec<u8>>, String> {
    let RespFrame::Array(Some(items)) = frame else {
        return Err("ERR protocol error".to_string());
    };

    let mut args = Vec::with_capacity(items.len());
    for item in items {
        match item {
            RespFrame::Bulk(Some(bytes)) => args.push(bytes),
            RespFrame::Simple(value) => args.push(value.into_bytes()),
            _ => return Err("ERR invalid argument type".to_string()),
        }
    }

    Ok(args)
}

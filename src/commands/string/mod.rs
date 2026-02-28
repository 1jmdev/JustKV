mod counter_ops;
mod multi_ops;
mod set_ops;

use crate::commands::util::Args;
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if let Some(response) = set_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = counter_ops::handle(store, command, args) {
        return Some(response);
    }
    multi_ops::handle(store, command, args)
}

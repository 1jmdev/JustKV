mod counter_ops;
mod expiry_ops;
mod get_set_ops;
mod length_ops;
mod multi_ops;

use crate::commands::util::Args;
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if let Some(response) = get_set_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = length_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = expiry_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = counter_ops::handle(store, command, args) {
        return Some(response);
    }
    multi_ops::handle(store, command, args)
}

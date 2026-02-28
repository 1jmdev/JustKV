mod core_ops;
mod counter_ops;
mod random_ops;
mod scan_ops;

use crate::commands::util::Args;
use crate::engine::store::Store;
use crate::protocol::types::RespFrame;

pub fn handle(store: &Store, command: &[u8], args: &Args) -> Option<RespFrame> {
    if let Some(response) = core_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = counter_ops::handle(store, command, args) {
        return Some(response);
    }
    if let Some(response) = scan_ops::handle(store, command, args) {
        return Some(response);
    }
    random_ops::handle(store, command, args)
}

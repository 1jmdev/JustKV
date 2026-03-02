mod claim_ops;
mod group_ops;
mod range_ops;
mod types;
mod write_ops;

use crate::engine::value::{Entry, StreamValue};

pub use types::{StreamRangeItem, XPendingSummary};

fn get_stream(entry: &Entry) -> Option<&StreamValue> {
    entry.as_stream()
}

fn get_stream_mut(entry: &mut Entry) -> Option<&mut StreamValue> {
    entry.as_stream_mut()
}

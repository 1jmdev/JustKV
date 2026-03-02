mod claim;
mod group;
mod range;
mod types;
mod write;

use crate::value::{Entry, StreamValue};

pub use types::{StreamRangeItem, XPendingSummary};

fn get_stream(entry: &Entry) -> Option<&StreamValue> {
    let _trace = profiler::scope("crates::engine::src::stream::get_stream");
    entry.as_stream()
}

fn get_stream_mut(entry: &mut Entry) -> Option<&mut StreamValue> {
    let _trace = profiler::scope("crates::engine::src::stream::get_stream_mut");
    entry.as_stream_mut()
}

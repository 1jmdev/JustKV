mod core;
mod moves;
mod multi_pop;

use crate::value::{Entry, ListValue};

fn get_list(entry: &Entry) -> Option<&ListValue> {
    let _trace = profiler::scope("crates::engine::src::list::get_list");
    entry.as_list()
}

fn get_list_mut(entry: &mut Entry) -> Option<&mut ListValue> {
    let _trace = profiler::scope("crates::engine::src::list::get_list_mut");
    entry.as_list_mut()
}

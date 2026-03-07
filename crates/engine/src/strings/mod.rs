mod bitmap;
mod core;
mod counter;
mod expiry;
mod hyperlog;
mod multi;
mod range;

use types::value::{CompactKey, Entry};

fn write_entry(shard: &mut super::Shard, key: &[u8], entry: Entry, ttl_deadline: Option<u64>) {
    let _trace = profiler::scope("engine::strings::write_entry");
    let compact_key = CompactKey::from_slice(key);
    shard.insert_entry(compact_key, entry, ttl_deadline);
}

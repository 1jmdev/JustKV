mod bitmap;
mod core;
mod counter;
mod expiry;
mod hyperlog;
mod multi;
mod range;

use crate::value::{CompactKey, Entry};

fn write_entry(shard: &mut super::Shard, key: &[u8], entry: Entry, ttl_deadline: Option<u64>) {
    let _trace = profiler::scope("engine::strings::write_entry");
    let compact_key = CompactKey::from_slice(key);

    if let Some(deadline) = ttl_deadline {
        shard.entries.insert(compact_key.clone(), entry);
        shard.set_ttl(compact_key, deadline);
    } else {
        // Only clear existing TTL if there actually is one — avoids a
        // needless hashmap lookup on every key write without a TTL.
        let had_ttl = shard.clear_ttl(key);
        shard.entries.insert(compact_key, entry);
        let _ = had_ttl;
    }
}

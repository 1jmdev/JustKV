mod core_ops;
mod counter_ops;
mod expiry_ops;
mod multi_ops;
mod range_ops;

use crate::engine::value::{CompactKey, Entry};

fn write_entry(shard: &mut super::Shard, key: &[u8], entry: Entry, ttl_deadline: Option<u64>) {
    let compact_key = CompactKey::from_slice(key);
    shard.entries.insert(compact_key.clone(), entry);

    if let Some(deadline) = ttl_deadline {
        shard.ttl.insert(compact_key, deadline);
    } else {
        shard.ttl.remove(compact_key.as_slice());
    }
}

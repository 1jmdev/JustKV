mod bitmap;
mod core;
mod counter;
mod delete;
mod digest;
mod expiry;
mod hyperlog;
mod lcs;
mod multi;
mod range;

use types::value::{CompactKey, Entry};

pub use counter::StringIntOpError;
pub use delete::StringDigestCondition;
pub use hyperlog::HyperLogLogError;

fn write_entry(shard: &mut super::Shard, key: &[u8], entry: Entry, ttl_deadline: Option<u64>) {
    let _trace = profiler::scope("engine::strings::write_entry");
    let compact_key = CompactKey::from_slice(key);
    shard.insert_entry(compact_key, entry, ttl_deadline);
}

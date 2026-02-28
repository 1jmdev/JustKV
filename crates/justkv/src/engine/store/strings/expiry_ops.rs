use std::time::Duration;

use crate::engine::store::{GetExMode, Store};
use crate::engine::value::CompactKey;

use super::super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired, unix_time_ms};

impl Store {
    pub fn getex(&self, key: &[u8], mode: GetExMode) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return None;
        }

        let value = shard.entries.get(key).map(|entry| entry.value.to_vec())?;

        match mode {
            GetExMode::KeepTtl => {}
            GetExMode::Persist => {
                shard.ttl.remove(key);
            }
            GetExMode::Ex(seconds) => {
                shard.ttl.insert(
                    CompactKey::from_slice(key),
                    deadline_from_ttl(Duration::from_secs(seconds)),
                );
            }
            GetExMode::Px(milliseconds) => {
                shard.ttl.insert(
                    CompactKey::from_slice(key),
                    deadline_from_ttl(Duration::from_millis(milliseconds)),
                );
            }
            GetExMode::ExAt(timestamp_sec) => {
                apply_getex_absolute_deadline(&mut shard, key, timestamp_sec.saturating_mul(1000));
            }
            GetExMode::PxAt(timestamp_ms) => {
                apply_getex_absolute_deadline(&mut shard, key, timestamp_ms);
            }
        }

        Some(value)
    }
}

fn apply_getex_absolute_deadline(shard: &mut super::super::Shard, key: &[u8], timestamp_ms: u64) {
    let now_unix_ms = unix_time_ms();
    if timestamp_ms <= now_unix_ms {
        shard.ttl.remove(key);
        shard.entries.remove(key);
        return;
    }

    let ttl_ms = timestamp_ms - now_unix_ms;
    shard.ttl.insert(
        CompactKey::from_slice(key),
        deadline_from_ttl(Duration::from_millis(ttl_ms)),
    );
}

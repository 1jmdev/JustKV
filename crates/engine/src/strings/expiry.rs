use std::time::Duration;

use super::super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired, unix_time_ms};
use crate::store::{GetExMode, Store};
use crate::value::CompactKey;

impl Store {
    pub fn getex(&self, key: &[u8], mode: GetExMode) -> Result<Option<Vec<u8>>, ()> {
        let _trace = profiler::scope("engine::strings::expiry::getex");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let value = match shard.entries.get::<[u8]>(key) {
            Some(entry) => match entry.as_string() {
                Some(value) => value.to_vec(),
                None => return Err(()),
            },
            None => return Ok(None),
        };

        match mode {
            GetExMode::KeepTtl => {}
            GetExMode::Persist => {
                let _ = shard.clear_ttl(key);
            }
            GetExMode::Ex(seconds) => {
                shard.set_ttl(
                    CompactKey::from_slice(key),
                    deadline_from_ttl(Duration::from_secs(seconds)),
                );
            }
            GetExMode::Px(milliseconds) => {
                shard.set_ttl(
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

        Ok(Some(value))
    }
}

fn apply_getex_absolute_deadline(shard: &mut super::super::Shard, key: &[u8], timestamp_ms: u64) {
    let _trace =
        profiler::scope("engine::strings::expiry::apply_getex_absolute_deadline");
    let now_unix_ms = unix_time_ms();
    if timestamp_ms <= now_unix_ms {
        let _ = shard.remove_key(key);
        return;
    }

    let ttl_ms = timestamp_ms - now_unix_ms;
    shard.set_ttl(
        CompactKey::from_slice(key),
        deadline_from_ttl(Duration::from_millis(ttl_ms)),
    );
}

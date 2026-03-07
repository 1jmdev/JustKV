use std::sync::atomic::{AtomicU64, Ordering};

use crate::store::Store;
use types::value::{CompactKey, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::get_set;

static RANDOM_COUNTER: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);

#[inline(always)]
fn random_seed(len: usize) -> u64 {
    RANDOM_COUNTER
        .fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed)
        .wrapping_add((len as u64).wrapping_mul(0xbf58476d1ce4e5b9))
}

#[inline(always)]
fn random_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58476d1ce4e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

impl Store {
    pub fn spop(&self, key: &[u8], count: usize) -> Result<Option<Vec<CompactKey>>, ()> {
        let _trace = profiler::scope("engine::set::random::spop");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get_mut::<[u8]>(key) else {
            return Ok(None);
        };
        let Entry::Set(set) = &mut entry.entry else {
            return Err(());
        };
        if set.is_empty() {
            return Ok(None);
        }

        let len = set.len();
        let take = count.min(len);
        if take == 0 {
            return Ok(Some(Vec::new()));
        }
        if take == len {
            let out: Vec<_> = set.drain(..).collect();
            let _ = shard.remove_key(key);
            return Ok(Some(out));
        }

        let mut state = random_seed(len);
        if take == 1 {
            let idx = (random_next(&mut state) as usize) % len;
            if let Some(member) = set.swap_remove_index(idx) {
                if set.is_empty() {
                    let _ = shard.remove_key(key);
                }
                return Ok(Some(vec![member]));
            }
            return Ok(Some(Vec::new()));
        }

        let mut out = Vec::with_capacity(take);
        for _ in 0..take {
            let idx = (random_next(&mut state) as usize) % set.len();
            let Some(member) = set.swap_remove_index(idx) else {
                break;
            };
            out.push(member);
        }

        if set.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(Some(out))
    }

    pub fn srandmember(&self, key: &[u8], count: i64) -> Result<Option<Vec<CompactKey>>, ()> {
        let _trace = profiler::scope("engine::set::random::srandmember");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get::<[u8]>(key) else {
            return Ok(None);
        };
        let Some(set) = get_set(entry) else {
            return Err(());
        };
        let len = set.len();
        if len == 0 {
            return Ok(None);
        }

        let mut state = random_seed(len);
        if count >= 0 {
            let take = (count as usize).min(len);
            if take == len {
                return Ok(Some(set.iter().cloned().collect()));
            }
            if take == 1 {
                let idx = (random_next(&mut state) as usize) % len;
                if let Some(member) = set.get_index(idx) {
                    return Ok(Some(vec![member.clone()]));
                }
                return Ok(Some(Vec::new()));
            }

            let mut selected = hashbrown::HashSet::with_capacity(take);
            let mut out = Vec::with_capacity(take);
            while out.len() < take {
                let idx = (random_next(&mut state) as usize) % len;
                if selected.insert(idx) {
                    if let Some(member) = set.get_index(idx) {
                        out.push(member.clone());
                    }
                }
            }
            Ok(Some(out))
        } else {
            let take = count.unsigned_abs() as usize;
            let mut out = Vec::with_capacity(take);
            for _ in 0..take {
                let idx = (random_next(&mut state) as usize) % len;
                if let Some(member) = set.get_index(idx) {
                    out.push(member.clone());
                }
            }
            Ok(Some(out))
        }
    }
}

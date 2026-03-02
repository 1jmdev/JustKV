use crate::engine::store::Store;
use crate::engine::value::{CompactArg, CompactKey, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_members, get_set, get_set_mut, new_set};

impl Store {
    pub fn sadd(&self, key: &[u8], members: &[CompactArg]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                Entry::Set(Box::new(new_set()))
            });
        let set = get_set_mut(entry).ok_or(())?;

        let mut added = 0;
        for member in members {
            if set.insert(CompactKey::from_slice(member.as_slice())) {
                added += 1;
            }
        }
        Ok(added)
    }

    pub fn srem(&self, key: &[u8], members: &[CompactArg]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let set = get_set_mut(entry).ok_or(())?;

        let mut removed = 0;
        for member in members {
            if set.remove(member.as_slice()) {
                removed += 1;
            }
        }

        if set.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(removed)
    }

    pub fn smembers(&self, key: &[u8]) -> Result<Vec<CompactKey>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let set = get_set(entry).ok_or(())?;
        Ok(collect_members(set))
    }

    pub fn sismember(&self, key: &[u8], member: &[u8]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let set = get_set(entry).ok_or(())?;
        Ok(set.contains(member) as i64)
    }

    pub fn scard(&self, key: &[u8]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let set = get_set(entry).ok_or(())?;
        Ok(set.len() as i64)
    }

    pub fn smove(&self, source: &[u8], destination: &[u8], member: &[u8]) -> Result<i64, ()> {
        let source_idx = self.shard_index(source);
        let destination_idx = self.shard_index(destination);
        let now_ms = monotonic_now_ms();

        if source_idx == destination_idx {
            let mut shard = self.shards[source_idx].write();
            let _ = purge_if_expired(&mut shard, source, now_ms);
            if source != destination {
                let _ = purge_if_expired(&mut shard, destination, now_ms);
            }
            return smove_inside_shard(&mut shard, source, destination, member);
        }

        let (first_idx, second_idx, source_is_first) = if source_idx < destination_idx {
            (source_idx, destination_idx, true)
        } else {
            (destination_idx, source_idx, false)
        };

        let mut first = self.shards[first_idx].write();
        let mut second = self.shards[second_idx].write();

        if source_is_first {
            let _ = purge_if_expired(&mut first, source, now_ms);
            let _ = purge_if_expired(&mut second, destination, now_ms);
            smove_across_shards(&mut first, source, &mut second, destination, member)
        } else {
            let _ = purge_if_expired(&mut second, source, now_ms);
            let _ = purge_if_expired(&mut first, destination, now_ms);
            smove_across_shards(&mut second, source, &mut first, destination, member)
        }
    }
}

fn smove_inside_shard(
    shard: &mut super::super::Shard,
    source: &[u8],
    destination: &[u8],
    member: &[u8],
) -> Result<i64, ()> {
    let Some(source_entry) = shard.entries.get_mut(source) else {
        return Ok(0);
    };
    let source_set = get_set_mut(source_entry).ok_or(())?;
    if !source_set.remove(member) {
        return Ok(0);
    }

    if source_set.is_empty() {
        let _ = shard.remove_key(source);
    }

    let destination_entry = shard
        .entries
        .get_or_insert_with(CompactKey::from_slice(destination), || {
            Entry::Set(Box::new(new_set()))
        });
    let destination_set = get_set_mut(destination_entry).ok_or(())?;
    destination_set.insert(CompactKey::from_slice(member));
    Ok(1)
}

fn smove_across_shards(
    source_shard: &mut super::super::Shard,
    source: &[u8],
    destination_shard: &mut super::super::Shard,
    destination: &[u8],
    member: &[u8],
) -> Result<i64, ()> {
    let Some(source_entry) = source_shard.entries.get_mut(source) else {
        return Ok(0);
    };
    let source_set = get_set_mut(source_entry).ok_or(())?;
    if !source_set.remove(member) {
        return Ok(0);
    }

    if source_set.is_empty() {
        let _ = source_shard.remove_key(source);
    }

    let destination_entry = destination_shard
        .entries
        .get_or_insert_with(CompactKey::from_slice(destination), || {
            Entry::Set(Box::new(new_set()))
        });
    let destination_set = get_set_mut(destination_entry).ok_or(())?;
    destination_set.insert(CompactKey::from_slice(member));
    Ok(1)
}

use parking_lot::RwLockReadGuard;

use crate::store::Store;
use crate::Shard;
use types::value::{CompactArg, CompactKey, Entry, SetValue};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_members, get_set, new_set};

impl Store {
    pub fn sinter(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let _trace = profiler::scope("engine::set::algebra::sinter");
        let now_ms = monotonic_now_ms();
        let guards = self.read_set_guards(keys, now_ms)?;

        let sets: Vec<Option<&SetValue>> = guards
            .iter()
            .zip(keys.iter())
            .map(|(guard, key)| resolve_set(guard, key.as_slice(), now_ms))
            .collect();

        if sets.iter().any(|s| s.is_none_or(|v| v.is_empty())) {
            return Ok(Vec::new());
        }

        let Some((first, rest)) = sets.split_first() else {
            return Ok(Vec::new());
        };
        let first = first.unwrap();
        let mut out: Vec<CompactKey> = first.iter().cloned().collect();
        out.retain(|member| {
            rest.iter()
                .all(|s| s.is_some_and(|set| set.contains(member.as_slice())))
        });
        Ok(out)
    }

    pub fn sunion(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let _trace = profiler::scope("engine::set::algebra::sunion");
        let now_ms = monotonic_now_ms();
        let guards = self.read_set_guards(keys, now_ms)?;

        let mut out = new_set();
        for (guard, key) in guards.iter().zip(keys.iter()) {
            if let Some(set) = resolve_set(guard, key.as_slice(), now_ms) {
                out.extend(set.iter().cloned());
            }
        }
        Ok(collect_members(&out))
    }

    pub fn sdiff(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let _trace = profiler::scope("engine::set::algebra::sdiff");
        let now_ms = monotonic_now_ms();
        let guards = self.read_set_guards(keys, now_ms)?;

        let sets: Vec<Option<&SetValue>> = guards
            .iter()
            .zip(keys.iter())
            .map(|(guard, key)| resolve_set(guard, key.as_slice(), now_ms))
            .collect();

        let Some((first, rest)) = sets.split_first() else {
            return Ok(Vec::new());
        };
        let Some(first) = first else {
            return Ok(Vec::new());
        };
        let mut out: Vec<CompactKey> = first.iter().cloned().collect();
        out.retain(|member| {
            rest.iter()
                .all(|s| s.is_none_or(|set| !set.contains(member.as_slice())))
        });
        Ok(out)
    }

    pub fn sinterstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::set::algebra::sinterstore");
        let result = self.sinter(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sunionstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::set::algebra::sunionstore");
        let result = self.sunion(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sdiffstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::set::algebra::sdiffstore");
        let result = self.sdiff(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sintercard(&self, keys: &[CompactArg], limit: Option<usize>) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::set::algebra::sintercard");
        let mut values = self.sinter(keys)?;
        if let Some(limit) = limit {
            values.truncate(limit);
        }
        Ok(values.len() as i64)
    }

    fn read_set_guards(
        &self,
        keys: &[CompactArg],
        _now_ms: u64,
    ) -> Result<Vec<RwLockReadGuard<'_, Shard>>, ()> {
        let _trace = profiler::scope("engine::set::algebra::set_snapshots");
        let mut guards = Vec::with_capacity(keys.len());
        for key in keys {
            let idx = self.shard_index(key.as_slice());
            guards.push(self.shards[idx].read());
        }
        Ok(guards)
    }
}

#[inline]
fn resolve_set<'g>(
    guard: &'g RwLockReadGuard<'_, Shard>,
    key: &[u8],
    now_ms: u64,
) -> Option<&'g SetValue> {
    let _trace = profiler::scope("engine::set::algebra::resolve_set");
    if is_expired(guard, key, now_ms) {
        return None;
    }
    guard
        .entries
        .get::<[u8]>(key)
        .and_then(|entry| get_set(entry))
}

impl Store {
    fn write_set_result(&self, destination: &[u8], values: Vec<CompactKey>) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::set::algebra::write_set_result");
        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, destination, now_ms);

        if values.is_empty() {
            let _ = shard.remove_key(destination);
            return Ok(0);
        }

        if shard
            .entries
            .get::<[u8]>(destination)
            .is_some_and(|entry| entry.kind() != "set")
        {
            return Err(());
        }

        let mut out = new_set();
        out.extend(values);
        shard.insert_entry(
            CompactKey::from_slice(destination),
            Entry::Set(Box::new(out)),
            None,
        );
        Ok(shard
            .entries
            .get::<[u8]>(destination)
            .and_then(|entry| entry.as_set())
            .map(|set| set.len() as i64)
            .unwrap_or(0))
    }
}

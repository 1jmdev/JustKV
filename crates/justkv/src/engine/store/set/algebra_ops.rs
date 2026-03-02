use crate::engine::store::Store;
use crate::engine::value::{CompactArg, CompactKey, Entry, SetValue};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_members, get_set, new_set};

impl Store {
    pub fn sinter(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let snapshots = self.set_snapshots(keys)?;
        if snapshots.iter().any(|set| set.is_empty()) {
            return Ok(Vec::new());
        }

        let Some((first, rest)) = snapshots.split_first() else {
            return Ok(Vec::new());
        };
        let mut out: Vec<CompactKey> = first.iter().cloned().collect();
        out.retain(|member| rest.iter().all(|set| set.contains(member.as_slice())));
        Ok(out)
    }

    pub fn sunion(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let snapshots = self.set_snapshots(keys)?;
        let mut out = new_set();
        for set in snapshots {
            out.extend(set.into_iter());
        }
        Ok(collect_members(&out))
    }

    pub fn sdiff(&self, keys: &[CompactArg]) -> Result<Vec<CompactKey>, ()> {
        let snapshots = self.set_snapshots(keys)?;
        let Some((first, rest)) = snapshots.split_first() else {
            return Ok(Vec::new());
        };
        let mut out: Vec<CompactKey> = first.iter().cloned().collect();
        out.retain(|member| rest.iter().all(|set| !set.contains(member.as_slice())));
        Ok(out)
    }

    pub fn sinterstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let result = self.sinter(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sunionstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let result = self.sunion(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sdiffstore(&self, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let result = self.sdiff(keys)?;
        self.write_set_result(destination, result)
    }

    pub fn sintercard(&self, keys: &[CompactArg], limit: Option<usize>) -> Result<i64, ()> {
        let mut values = self.sinter(keys)?;
        if let Some(limit) = limit {
            values.truncate(limit);
        }
        Ok(values.len() as i64)
    }

    fn set_snapshots(&self, keys: &[CompactArg]) -> Result<Vec<SetValue>, ()> {
        let mut snapshots = Vec::with_capacity(keys.len());
        let now_ms = monotonic_now_ms();
        for key in keys {
            let idx = self.shard_index(key.as_slice());
            let shard = self.shards[idx].read();
            if is_expired(&shard, key.as_slice(), now_ms) {
                snapshots.push(new_set());
                continue;
            }

            match shard.entries.get::<[u8]>(key.as_slice()) {
                None => snapshots.push(new_set()),
                Some(entry) => {
                    let Some(set) = get_set(entry) else {
                        return Err(());
                    };
                    snapshots.push(set.clone());
                }
            }
        }
        Ok(snapshots)
    }

    fn write_set_result(&self, destination: &[u8], values: Vec<CompactKey>) -> Result<i64, ()> {
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
        shard.entries.insert(
            CompactKey::from_slice(destination),
            Entry::Set(Box::new(out)),
        );
        let _ = shard.clear_ttl(destination);
        Ok(shard
            .entries
            .get::<[u8]>(destination)
            .and_then(|entry| entry.as_set())
            .map(|set| set.len() as i64)
            .unwrap_or(0))
    }
}

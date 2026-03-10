use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::digest::xxh3_hex;
use crate::store::Store;
use types::value::CompactValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringDigestCondition<'a> {
    Eq(&'a [u8]),
    Ne(&'a [u8]),
    DigestEq(&'a [u8]),
    DigestNe(&'a [u8]),
}

impl Store {
    pub fn delex(
        &self,
        key: &[u8],
        condition: Option<StringDigestCondition<'_>>,
    ) -> Result<bool, ()> {
        let _trace = profiler::scope("engine::strings::delete::delex");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(false);
        }

        let should_delete = match condition {
            None => shard.entries.contains_key(key),
            Some(condition) => {
                let Some(entry) = shard.entries.get::<[u8]>(key) else {
                    return Ok(false);
                };
                let Some(value) = entry.as_string() else {
                    return Err(());
                };
                match condition {
                    StringDigestCondition::Eq(expected) => value.as_slice() == expected,
                    StringDigestCondition::Ne(expected) => value.as_slice() != expected,
                    StringDigestCondition::DigestEq(expected) => xxh3_hex_eq(value, expected),
                    StringDigestCondition::DigestNe(expected) => !xxh3_hex_eq(value, expected),
                }
            }
        };

        if !should_delete {
            return Ok(false);
        }

        Ok(shard.remove_key(key).is_some())
    }
}

pub fn xxh3_hex_eq(value: &CompactValue, expected: &[u8]) -> bool {
    xxh3_hex(value).as_slice() == expected
}

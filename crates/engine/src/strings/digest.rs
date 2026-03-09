use crate::store::Store;
use types::value::CompactValue;
use xxhash_rust::xxh3::xxh3_64;

use super::super::helpers::{get_live_entry, monotonic_now_ms, purge_if_expired, unix_time_ms};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringDigestCondition<'a> {
    Eq(&'a [u8]),
    Ne(&'a [u8]),
    DigestEq(&'a [u8]),
    DigestNe(&'a [u8]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MSetExExistCondition {
    Any,
    Nx,
    Xx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedTtl {
    None,
    RelativeMs(u64),
    AbsoluteUnixMs(u64),
    Keep,
}

impl Store {
    pub fn digest(&self, key: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        let _trace = profiler::scope("engine::strings::digest::digest");
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let Some(entry) = get_live_entry(&shard, key, now_ms) else {
            return Ok(None);
        };
        let Some(value) = entry.as_string() else {
            return Err(());
        };
        Ok(Some(xxh3_hex(value)))
    }

    pub fn delex(
        &self,
        key: &[u8],
        condition: Option<StringDigestCondition<'_>>,
    ) -> Result<bool, ()> {
        let _trace = profiler::scope("engine::strings::digest::delex");
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

    pub fn msetex(
        &self,
        pairs: &[(types::value::CompactArg, types::value::CompactArg)],
        condition: MSetExExistCondition,
        ttl: SharedTtl,
    ) -> i64 {
        let _trace = profiler::scope("engine::strings::digest::msetex");
        let now_ms = monotonic_now_ms();

        for (key, _) in pairs {
            let idx = self.shard_index(key.as_slice());
            let mut shard = self.shards[idx].write();
            let expired = purge_if_expired(&mut shard, key.as_slice(), now_ms);
            let exists = !expired && shard.entries.contains_key(key.as_slice());
            match condition {
                MSetExExistCondition::Any => {}
                MSetExExistCondition::Nx if exists => return 0,
                MSetExExistCondition::Xx if !exists => return 0,
                _ => {}
            }
        }

        let shared_deadline = ttl.shared_deadline_ms();
        for (key, value) in pairs {
            let idx = self.shard_index(key.as_slice());
            let mut shard = self.shards[idx].write();
            let preserve_deadline = match ttl {
                SharedTtl::Keep => shard.ttl_deadline(key.as_slice()),
                _ => None,
            };
            let deadline = preserve_deadline.or(shared_deadline);
            shard.insert_entry(
                types::value::CompactKey::from_slice(key.as_slice()),
                types::value::Entry::String(CompactValue::from_slice(value.as_slice())),
                deadline,
            );
        }

        1
    }
}

impl SharedTtl {
    fn shared_deadline_ms(self) -> Option<u64> {
        match self {
            Self::None | Self::Keep => None,
            Self::RelativeMs(milliseconds) => Some(super::super::helpers::deadline_from_ttl(
                std::time::Duration::from_millis(milliseconds),
            )),
            Self::AbsoluteUnixMs(unix_ms) => {
                let now_unix_ms = unix_time_ms();
                if unix_ms <= now_unix_ms {
                    Some(super::super::helpers::deadline_from_ttl(
                        std::time::Duration::from_millis(0),
                    ))
                } else {
                    Some(super::super::helpers::deadline_from_ttl(
                        std::time::Duration::from_millis(unix_ms - now_unix_ms),
                    ))
                }
            }
        }
    }
}

pub fn xxh3_hex(value: &CompactValue) -> Vec<u8> {
    let digest = xxh3_64(value.as_slice());
    let mut out = Vec::with_capacity(16);
    for shift in (0..8).rev() {
        let byte = ((digest >> (shift * 8)) & 0xff) as u8;
        out.push(nibble_to_hex(byte >> 4));
        out.push(nibble_to_hex(byte & 0x0f));
    }
    out
}

fn xxh3_hex_eq(value: &CompactValue, expected: &[u8]) -> bool {
    xxh3_hex(value).as_slice() == expected
}

#[inline]
fn nibble_to_hex(value: u8) -> u8 {
    if value < 10 {
        b'0' + value
    } else {
        b'a' + (value - 10)
    }
}

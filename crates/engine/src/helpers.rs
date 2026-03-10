use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{Shard, StoredEntry};

static START: OnceLock<Instant> = OnceLock::new();
static CACHED_TIME_MS: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn saturating_u128_to_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[inline(always)]
fn saturating_u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn monotonic_now_ms() -> u64 {
    let _trace = profiler::scope("engine::helpers::monotonic_now_ms");
    let cached = CACHED_TIME_MS.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }

    let now = START.get_or_init(Instant::now).elapsed().as_millis();
    let now = saturating_u128_to_u64(now);
    CACHED_TIME_MS.store(now, Ordering::Relaxed);
    now
}

pub(super) fn refresh_monotonic_now_ms() {
    let _trace = profiler::scope("engine::helpers::refresh_monotonic_now_ms");
    let now = START.get_or_init(Instant::now).elapsed().as_millis();
    let now = saturating_u128_to_u64(now);
    CACHED_TIME_MS.store(now, Ordering::Relaxed);
}

pub(super) fn deadline_from_ttl(ttl: Duration) -> u64 {
    let _trace = profiler::scope("engine::helpers::deadline_from_ttl");
    let ttl_ms = saturating_u128_to_u64(ttl.as_millis());
    monotonic_now_ms().saturating_add(ttl_ms)
}

pub(super) fn remaining_ttl_ms(deadline_ms: u64) -> i64 {
    let _trace = profiler::scope("engine::helpers::remaining_ttl_ms");
    if deadline_ms == 0 {
        return -1;
    }

    let now_ms = monotonic_now_ms();
    if deadline_ms <= now_ms {
        0
    } else {
        saturating_u64_to_i64(deadline_ms - now_ms)
    }
}

pub(super) fn purge_if_expired(shard: &mut Shard, key: &[u8], now_ms: u64) -> bool {
    let _trace = profiler::scope("engine::helpers::purge_if_expired");
    let expired = is_expired(shard, key, now_ms);
    if expired {
        let _ = shard.remove_key(key);
    }
    expired
}

#[inline(always)]
pub(super) fn get_live_entry<'a>(
    shard: &'a Shard,
    key: &[u8],
    now_ms: u64,
) -> Option<&'a StoredEntry> {
    let entry = shard.entries.get(key)?;
    (!shard.is_expired(key, now_ms)).then_some(entry)
}

pub(super) fn is_expired(shard: &Shard, key: &[u8], now_ms: u64) -> bool {
    let _trace = profiler::scope("engine::helpers::is_expired");
    if !shard.has_ttls() {
        return false;
    }
    shard.is_expired(key, now_ms)
}

pub(super) fn unix_time_ms() -> u64 {
    let _trace = profiler::scope("engine::helpers::unix_time_ms");
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::Shard;

static START: OnceLock<Instant> = OnceLock::new();
static CACHED_TIME_MS: AtomicU64 = AtomicU64::new(0);

pub(super) fn monotonic_now_ms() -> u64 {
    let cached = CACHED_TIME_MS.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }

    let now = START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    CACHED_TIME_MS.store(now, Ordering::Relaxed);
    now
}

pub(super) fn refresh_monotonic_now_ms() {
    let now = START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    CACHED_TIME_MS.store(now, Ordering::Relaxed);
}

pub(super) fn deadline_from_ttl(ttl: Duration) -> u64 {
    monotonic_now_ms().saturating_add(ttl.as_millis().try_into().unwrap_or(u64::MAX))
}

pub(super) fn remaining_ttl_ms(deadline_ms: u64) -> i64 {
    if deadline_ms == 0 {
        return -1;
    }

    let now_ms = monotonic_now_ms();
    if deadline_ms <= now_ms {
        0
    } else {
        (deadline_ms - now_ms).try_into().unwrap_or(i64::MAX)
    }
}

pub(super) fn purge_if_expired(shard: &mut Shard, key: &[u8], now_ms: u64) -> bool {
    let expired = is_expired(shard, key, now_ms);
    if expired {
        shard.ttl.remove(key);
        shard.entries.remove(key);
    }
    expired
}

pub(super) fn is_expired(shard: &Shard, key: &[u8], now_ms: u64) -> bool {
    shard
        .ttl
        .get(key)
        .copied()
        .is_some_and(|deadline| now_ms >= deadline)
}

pub(super) fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::hash::BuildHasher;

use hashbrown::{HashMap, HashSet};
use rapidhash::fast::RandomState;
use types::value::CompactArg;

use crate::pattern::wildcard_match;

type SubscriberMap = HashMap<u64, SharedPubSubSink, RandomState>;

pub type SharedPubSubSink = Arc<dyn PubSubSink>;

#[derive(Clone)]
pub struct PubSubHub {
    shards: Arc<Vec<parking_lot::RwLock<PubSubShard>>>,
    shard_mask: usize,
    hash_builder: RandomState,
    next_id: Arc<AtomicU64>,
    notify_mask: Arc<AtomicU16>,
    notify_enabled: Arc<AtomicBool>,
}

struct PubSubShard {
    channels: HashMap<Vec<u8>, SubscriberMap, RandomState>,
    shard_channels: HashMap<Vec<u8>, SubscriberMap, RandomState>,
    patterns_by_prefix: HashMap<Vec<u8>, HashMap<Vec<u8>, SubscriberMap, RandomState>, RandomState>,
}

pub struct ConnectionPubSub {
    id: u64,
    channels: HashSet<Vec<u8>, RandomState>,
    patterns: HashSet<Vec<u8>, RandomState>,
    shard_channels: HashSet<Vec<u8>, RandomState>,
}

#[derive(Clone)]
pub enum PubSubMessage {
    Message {
        channel: CompactArg,
        payload: CompactArg,
    },
    PatternMessage {
        pattern: CompactArg,
        channel: CompactArg,
        payload: CompactArg,
    },
    ShardMessage {
        channel: CompactArg,
        payload: CompactArg,
    },
}

pub trait PubSubSink: Send + Sync {
    fn push(&self, message: PubSubMessage) -> bool;
}

impl PubSubHub {
    pub fn new() -> Self {
        let shard_count = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
            .saturating_mul(4)
            .max(1)
            .next_power_of_two();

        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shards.push(parking_lot::RwLock::new(PubSubShard {
                channels: HashMap::with_hasher(RandomState::new()),
                shard_channels: HashMap::with_hasher(RandomState::new()),
                patterns_by_prefix: HashMap::with_hasher(RandomState::new()),
            }));
        }

        Self {
            shards: Arc::new(shards),
            shard_mask: shard_count - 1,
            hash_builder: RandomState::new(),
            next_id: Arc::new(AtomicU64::new(1)),
            notify_mask: Arc::new(AtomicU16::new(0)),
            notify_enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn next_connection_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn subscribe(&self, id: u64, channel: &[u8], sink: &SharedPubSubSink) -> bool {
        let idx = self.shard_index(channel);
        let mut shard = self.shards[idx].write();
        let subscribers = shard.channels.entry(channel.to_vec()).or_default();
        subscribers.insert(id, Arc::clone(sink)).is_none()
    }

    pub fn unsubscribe(&self, id: u64, channel: &[u8]) -> bool {
        let idx = self.shard_index(channel);
        let mut shard = self.shards[idx].write();
        let Some(subscribers) = shard.channels.get_mut(channel) else {
            return false;
        };

        let removed = subscribers.remove(&id).is_some();
        if subscribers.is_empty() {
            shard.channels.remove(channel);
        }
        removed
    }

    pub fn psubscribe(&self, id: u64, pattern: &[u8], sink: &SharedPubSubSink) -> bool {
        let prefix = pattern_prefix(pattern);
        let idx = self.shard_index(prefix.as_slice());
        let mut shard = self.shards[idx].write();
        let subscribers = shard
            .patterns_by_prefix
            .entry(prefix)
            .or_default()
            .entry(pattern.to_vec())
            .or_default();

        subscribers.insert(id, Arc::clone(sink)).is_none()
    }

    pub fn punsubscribe(&self, id: u64, pattern: &[u8]) -> bool {
        let prefix = pattern_prefix(pattern);
        let idx = self.shard_index(prefix.as_slice());
        let mut shard = self.shards[idx].write();
        let Some(patterns) = shard.patterns_by_prefix.get_mut(prefix.as_slice()) else {
            return false;
        };
        let Some(subscribers) = patterns.get_mut(pattern) else {
            return false;
        };

        let removed = subscribers.remove(&id).is_some();
        if subscribers.is_empty() {
            patterns.remove(pattern);
        }
        if patterns.is_empty() {
            shard.patterns_by_prefix.remove(prefix.as_slice());
        }
        removed
    }

    pub fn ssubscribe(&self, id: u64, channel: &[u8], sink: &SharedPubSubSink) -> bool {
        let idx = self.shard_index(channel);
        let mut shard = self.shards[idx].write();
        let subscribers = shard.shard_channels.entry(channel.to_vec()).or_default();
        subscribers.insert(id, Arc::clone(sink)).is_none()
    }

    pub fn sunsubscribe(&self, id: u64, channel: &[u8]) -> bool {
        let idx = self.shard_index(channel);
        let mut shard = self.shards[idx].write();
        let Some(subscribers) = shard.shard_channels.get_mut(channel) else {
            return false;
        };

        let removed = subscribers.remove(&id).is_some();
        if subscribers.is_empty() {
            shard.shard_channels.remove(channel);
        }
        removed
    }

    pub fn cleanup_connection(&self, id: u64) {
        for shard in self.shards.iter() {
            let mut shard = shard.write();
            shard.channels.retain(|_, subscribers| {
                subscribers.remove(&id);
                !subscribers.is_empty()
            });
            shard.shard_channels.retain(|_, subscribers| {
                subscribers.remove(&id);
                !subscribers.is_empty()
            });
            shard.patterns_by_prefix.retain(|_, patterns| {
                patterns.retain(|_, subscribers| {
                    subscribers.remove(&id);
                    !subscribers.is_empty()
                });
                !patterns.is_empty()
            });
        }
    }

    pub fn publish(&self, channel: &[u8], payload: &[u8]) -> i64 {
        let channel_subscribers = {
            let idx = self.shard_index(channel);
            let shard = self.shards[idx].read();
            shard
                .channels
                .get(channel)
                .map(|subs| subs.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        let mut pattern_subscribers = Vec::new();
        for prefix_len in 0..=channel.len() {
            let prefix = &channel[..prefix_len];
            let idx = self.shard_index(prefix);
            let shard = self.shards[idx].read();
            let Some(patterns) = shard.patterns_by_prefix.get(prefix) else {
                continue;
            };

            for (pattern, subscribers) in patterns {
                if !wildcard_match(pattern, channel) {
                    continue;
                }
                pattern_subscribers.push((
                    pattern.clone(),
                    subscribers.values().cloned().collect::<Vec<_>>(),
                ));
            }
        }

        let channel = CompactArg::from_slice(channel);
        let payload = CompactArg::from_slice(payload);
        let mut delivered = 0_i64;

        for sink in channel_subscribers {
            if sink.push(PubSubMessage::Message {
                channel: channel.clone(),
                payload: payload.clone(),
            }) {
                delivered += 1;
            }
        }

        for (pattern, subscribers) in pattern_subscribers {
            let pattern = CompactArg::from_vec(pattern);
            for sink in subscribers {
                if sink.push(PubSubMessage::PatternMessage {
                    pattern: pattern.clone(),
                    channel: channel.clone(),
                    payload: payload.clone(),
                }) {
                    delivered += 1;
                }
            }
        }

        delivered
    }

    pub fn spublish(&self, channel: &[u8], payload: &[u8]) -> i64 {
        let subscribers = {
            let idx = self.shard_index(channel);
            let shard = self.shards[idx].read();
            shard
                .shard_channels
                .get(channel)
                .map(|subs| subs.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        let channel = CompactArg::from_slice(channel);
        let payload = CompactArg::from_slice(payload);
        let mut delivered = 0_i64;
        for sink in subscribers {
            if sink.push(PubSubMessage::ShardMessage {
                channel: channel.clone(),
                payload: payload.clone(),
            }) {
                delivered += 1;
            }
        }
        delivered
    }

    pub fn pubsub_channels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        let mut channels = Vec::new();
        for shard in self.shards.iter() {
            let shard = shard.read();
            channels.extend(
                shard
                    .channels
                    .keys()
                    .filter(|channel| {
                        pattern.is_none_or(|matcher| wildcard_match(matcher, channel))
                    })
                    .cloned(),
            );
        }
        channels.sort_unstable();
        channels
    }

    pub fn pubsub_numsub(&self, channels: &[Vec<u8>]) -> Vec<(Vec<u8>, i64)> {
        channels
            .iter()
            .map(|channel| {
                let idx = self.shard_index(channel);
                let shard = self.shards[idx].read();
                let count = shard
                    .channels
                    .get(channel.as_slice())
                    .map_or(0_i64, |subscribers| subscribers.len() as i64);
                (channel.clone(), count)
            })
            .collect()
    }

    pub fn pubsub_numpat(&self) -> i64 {
        self.shards
            .iter()
            .map(|shard| {
                let shard = shard.read();
                shard
                    .patterns_by_prefix
                    .values()
                    .map(|patterns| patterns.len() as i64)
                    .sum::<i64>()
            })
            .sum()
    }

    pub fn pubsub_shardchannels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        let mut channels = Vec::new();
        for shard in self.shards.iter() {
            let shard = shard.read();
            channels.extend(
                shard
                    .shard_channels
                    .keys()
                    .filter(|channel| {
                        pattern.is_none_or(|matcher| wildcard_match(matcher, channel))
                    })
                    .cloned(),
            );
        }
        channels.sort_unstable();
        channels
    }

    pub fn pubsub_shardnumsub(&self, channels: &[Vec<u8>]) -> Vec<(Vec<u8>, i64)> {
        channels
            .iter()
            .map(|channel| {
                let idx = self.shard_index(channel);
                let shard = self.shards[idx].read();
                let count = shard
                    .shard_channels
                    .get(channel.as_slice())
                    .map_or(0_i64, |subscribers| subscribers.len() as i64);
                (channel.clone(), count)
            })
            .collect()
    }

    pub fn set_notify_flags(&self, flags: &[u8]) -> Result<(), ()> {
        let mask = flags_to_mask(flags)?;
        self.notify_mask.store(mask, Ordering::Relaxed);
        let has_output_target = (mask & (FLAG_KEYSPACE | FLAG_KEYEVENT)) != 0;
        let has_event_classes = (mask
            & (FLAG_A | FLAG_G | FLAG_S | FLAG_H | FLAG_Z | FLAG_L | FLAG_DOLLAR | FLAG_X))
            != 0;
        self.notify_enabled
            .store(has_output_target && has_event_classes, Ordering::Relaxed);
        Ok(())
    }

    pub fn get_notify_flags(&self) -> Vec<u8> {
        mask_to_flags(self.notify_mask.load(Ordering::Relaxed))
    }

    pub fn keyspace_notifications_enabled(&self) -> bool {
        self.notify_enabled.load(Ordering::Relaxed)
    }

    pub fn emit_keyspace_event(&self, event: &[u8], key: &[u8], class: u8) {
        let mask = self.notify_mask.load(Ordering::Relaxed);
        if !notifications_enabled(mask, class) {
            return;
        }

        if notifications_enabled_keyspace(mask) {
            let channel = make_notification_channel(b"__keyspace@0__:", key);
            self.publish(&channel, event);
        }
        if notifications_enabled_keyevent(mask) {
            let channel = make_notification_channel(b"__keyevent@0__:", event);
            self.publish(&channel, key);
        }
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}

impl Default for PubSubHub {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPubSub {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            channels: HashSet::with_hasher(RandomState::new()),
            patterns: HashSet::with_hasher(RandomState::new()),
            shard_channels: HashSet::with_hasher(RandomState::new()),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn subscribe(&mut self, hub: &PubSubHub, channel: &[u8], sink: &SharedPubSubSink) {
        if self.channels.insert(channel.to_vec()) {
            hub.subscribe(self.id, channel, sink);
        }
    }

    pub fn unsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> bool {
        if self.channels.remove(channel) {
            hub.unsubscribe(self.id, channel);
            true
        } else {
            false
        }
    }

    pub fn psubscribe(&mut self, hub: &PubSubHub, pattern: &[u8], sink: &SharedPubSubSink) {
        if self.patterns.insert(pattern.to_vec()) {
            hub.psubscribe(self.id, pattern, sink);
        }
    }

    pub fn punsubscribe(&mut self, hub: &PubSubHub, pattern: &[u8]) -> bool {
        if self.patterns.remove(pattern) {
            hub.punsubscribe(self.id, pattern);
            true
        } else {
            false
        }
    }

    pub fn ssubscribe(&mut self, hub: &PubSubHub, channel: &[u8], sink: &SharedPubSubSink) {
        if self.shard_channels.insert(channel.to_vec()) {
            hub.ssubscribe(self.id, channel, sink);
        }
    }

    pub fn sunsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> bool {
        if self.shard_channels.remove(channel) {
            hub.sunsubscribe(self.id, channel);
            true
        } else {
            false
        }
    }

    pub fn unsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let channels = self.channels.drain().collect::<Vec<_>>();
        for channel in &channels {
            hub.unsubscribe(self.id, channel);
        }
        channels
    }

    pub fn punsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let patterns = self.patterns.drain().collect::<Vec<_>>();
        for pattern in &patterns {
            hub.punsubscribe(self.id, pattern);
        }
        patterns
    }

    pub fn sunsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let channels = self.shard_channels.drain().collect::<Vec<_>>();
        for channel in &channels {
            hub.sunsubscribe(self.id, channel);
        }
        channels
    }

    pub fn subscription_count(&self) -> i64 {
        (self.channels.len() + self.patterns.len() + self.shard_channels.len()) as i64
    }
}

const FLAG_G: u16 = 1 << 0;
const FLAG_DOLLAR: u16 = 1 << 1;
const FLAG_L: u16 = 1 << 2;
const FLAG_S: u16 = 1 << 3;
const FLAG_H: u16 = 1 << 4;
const FLAG_Z: u16 = 1 << 5;
const FLAG_X: u16 = 1 << 6;
const FLAG_KEYEVENT: u16 = 1 << 7;
const FLAG_KEYSPACE: u16 = 1 << 8;
const FLAG_A: u16 = 1 << 9;

fn flag_to_mask(flag: u8) -> Option<u16> {
    match flag {
        b'g' => Some(FLAG_G),
        b'$' => Some(FLAG_DOLLAR),
        b'l' => Some(FLAG_L),
        b's' => Some(FLAG_S),
        b'h' => Some(FLAG_H),
        b'z' => Some(FLAG_Z),
        b'x' => Some(FLAG_X),
        b'e' => Some(FLAG_KEYEVENT),
        b'K' => Some(FLAG_KEYSPACE),
        b'E' => Some(FLAG_KEYEVENT),
        b'A' => Some(FLAG_A),
        _ => None,
    }
}

fn flags_to_mask(flags: &[u8]) -> Result<u16, ()> {
    let mut mask = 0_u16;
    for &flag in flags {
        let bit = flag_to_mask(flag).ok_or(())?;
        mask |= bit;
    }
    Ok(mask)
}

fn mask_to_flags(mask: u16) -> Vec<u8> {
    let mut out = Vec::new();
    if mask & FLAG_A != 0 {
        out.push(b'A');
    }
    if mask & FLAG_G != 0 {
        out.push(b'g');
    }
    if mask & FLAG_DOLLAR != 0 {
        out.push(b'$');
    }
    if mask & FLAG_L != 0 {
        out.push(b'l');
    }
    if mask & FLAG_S != 0 {
        out.push(b's');
    }
    if mask & FLAG_H != 0 {
        out.push(b'h');
    }
    if mask & FLAG_Z != 0 {
        out.push(b'z');
    }
    if mask & FLAG_X != 0 {
        out.push(b'x');
    }
    if mask & FLAG_KEYSPACE != 0 {
        out.push(b'K');
    }
    if mask & FLAG_KEYEVENT != 0 {
        out.push(b'E');
    }
    out
}

fn notifications_enabled(mask: u16, class: u8) -> bool {
    (mask & FLAG_A) != 0 || flag_to_mask(class).is_some_and(|bit| (mask & bit) != 0)
}

fn notifications_enabled_keyspace(mask: u16) -> bool {
    (mask & FLAG_KEYSPACE) != 0
}

fn notifications_enabled_keyevent(mask: u16) -> bool {
    (mask & FLAG_KEYEVENT) != 0
}

fn pattern_prefix(pattern: &[u8]) -> Vec<u8> {
    let mut index = 0;
    while index < pattern.len() {
        match pattern[index] {
            b'*' | b'?' | b'[' => break,
            b'\\' if index + 1 < pattern.len() => index += 2,
            _ => index += 1,
        }
    }
    pattern[..index.min(pattern.len())].to_vec()
}

fn make_notification_channel(prefix: &[u8], value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(prefix.len() + value.len());
    out.extend_from_slice(prefix);
    out.extend_from_slice(value);
    out
}

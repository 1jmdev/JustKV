use std::sync::Arc;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};

use ahash::{AHashMap, AHashSet, RandomState};
use bytes::{Bytes, BytesMut};
use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use protocol::encoder::encode;
use protocol::types::{BulkData, RespFrame};

type SubscriberMap = AHashMap<u64, UnboundedSender<RespFrame>>;

#[derive(Clone)]
pub struct PubSubHub {
    shards: Arc<Vec<RwLock<PubSubShard>>>,
    shard_mask: usize,
    hash_builder: RandomState,
    next_id: Arc<AtomicU64>,
    notify_mask: Arc<AtomicU16>,
}

struct PubSubShard {
    channels: AHashMap<Vec<u8>, SubscriberMap>,
    patterns_by_prefix: AHashMap<Vec<u8>, AHashMap<Vec<u8>, SubscriberMap>>,
}

impl PubSubHub {
    pub fn new() -> Self {
        let _trace = profiler::scope("server::pubsub::new");
        let shard_count = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
            .saturating_mul(4)
            .max(1)
            .next_power_of_two();

        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shards.push(RwLock::new(PubSubShard {
                channels: AHashMap::new(),
                patterns_by_prefix: AHashMap::new(),
            }));
        }

        Self {
            shards: Arc::new(shards),
            shard_mask: shard_count - 1,
            hash_builder: RandomState::new(),
            next_id: Arc::new(AtomicU64::new(1)),
            notify_mask: Arc::new(AtomicU16::new(0)),
        }
    }

    pub fn next_connection_id(&self) -> u64 {
        let _trace = profiler::scope("server::pubsub::next_connection_id");
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn subscribe(&self, id: u64, channel: &[u8], tx: &UnboundedSender<RespFrame>) -> bool {
        let _trace = profiler::scope("server::pubsub::subscribe");
        let idx = self.shard_index(channel);
        let mut shard = self.shards[idx].write();
        let subscribers = shard.channels.entry(channel.to_vec()).or_default();
        subscribers.insert(id, tx.clone()).is_none()
    }

    pub fn unsubscribe(&self, id: u64, channel: &[u8]) -> bool {
        let _trace = profiler::scope("server::pubsub::unsubscribe");
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

    pub fn psubscribe(&self, id: u64, pattern: &[u8], tx: &UnboundedSender<RespFrame>) -> bool {
        let _trace = profiler::scope("server::pubsub::psubscribe");
        let prefix = pattern_prefix(pattern);
        let idx = self.shard_index(prefix.as_slice());
        let mut shard = self.shards[idx].write();
        let subscribers = shard
            .patterns_by_prefix
            .entry(prefix)
            .or_default()
            .entry(pattern.to_vec())
            .or_default();

        subscribers.insert(id, tx.clone()).is_none()
    }

    pub fn punsubscribe(&self, id: u64, pattern: &[u8]) -> bool {
        let _trace = profiler::scope("server::pubsub::punsubscribe");
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

    pub fn cleanup_connection(&self, id: u64) {
        let _trace = profiler::scope("server::pubsub::cleanup_connection");
        for shard in self.shards.iter() {
            let mut shard = shard.write();
            shard.channels.retain(|_, subscribers| {
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
        let _trace = profiler::scope("server::pubsub::publish");
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

        let mut delivered = 0_i64;
        if !channel_subscribers.is_empty() {
            let frame = RespFrame::PreEncoded(encode_message_frame(channel, payload));
            for tx in channel_subscribers {
                if tx.send(frame.clone()).is_ok() {
                    delivered += 1;
                }
            }
        }

        for (pattern, subscribers) in pattern_subscribers {
            let frame = RespFrame::PreEncoded(encode_pmessage_frame(&pattern, channel, payload));
            for tx in subscribers {
                if tx.send(frame.clone()).is_ok() {
                    delivered += 1;
                }
            }
        }

        delivered
    }

    pub fn set_notify_flags(&self, flags: &[u8]) -> Result<(), ()> {
        let _trace = profiler::scope("server::pubsub::set_notify_flags");
        let mask = flags_to_mask(flags)?;
        self.notify_mask.store(mask, Ordering::Relaxed);
        Ok(())
    }

    pub fn get_notify_flags(&self) -> Vec<u8> {
        let _trace = profiler::scope("server::pubsub::get_notify_flags");
        mask_to_flags(self.notify_mask.load(Ordering::Relaxed))
    }

    pub fn keyspace_notifications_enabled(&self) -> bool {
        let _trace = profiler::scope("server::pubsub::keyspace_notifications_enabled");
        let mask = self.notify_mask.load(Ordering::Relaxed);
        let has_output_target = (mask & (FLAG_KEYSPACE | FLAG_KEYEVENT)) != 0;
        let has_event_classes = (mask
            & (FLAG_A | FLAG_G | FLAG_S | FLAG_H | FLAG_Z | FLAG_L | FLAG_DOLLAR | FLAG_X))
            != 0;
        has_output_target && has_event_classes
    }

    pub fn emit_keyspace_event(&self, event: &[u8], key: &[u8], class: u8) {
        let _trace = profiler::scope("server::pubsub::emit_keyspace_event");
        let mask = self.notify_mask.load(Ordering::Relaxed);
        if !notifications_enabled(mask, class) {
            return;
        }

        if notifications_enabled_keyspace(mask) {
            let channel = make_notification_channel(b"__keyspace@0__:", key);
            let _ = self.publish(&channel, event);
        }
        if notifications_enabled_keyevent(mask) {
            let channel = make_notification_channel(b"__keyevent@0__:", event);
            let _ = self.publish(&channel, key);
        }
    }

    pub fn pubsub_channels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        let _trace = profiler::scope("server::pubsub::pubsub_channels");
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
        let _trace = profiler::scope("server::pubsub::pubsub_numsub");
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
        let _trace = profiler::scope("server::pubsub::pubsub_numpat");
        self.shards
            .iter()
            .map(|shard| {
                let shard = shard.read();
                shard
                    .patterns_by_prefix
                    .values()
                    .map(|patterns| {
                        patterns
                            .values()
                            .map(|subscribers| subscribers.len() as i64)
                            .sum::<i64>()
                    })
                    .sum::<i64>()
            })
            .sum()
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let _trace = profiler::scope("server::pubsub::shard_index");
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}

pub struct ConnectionPubSub {
    pub id: u64,
    channels: AHashSet<Vec<u8>>,
    patterns: AHashSet<Vec<u8>>,
}

impl ConnectionPubSub {
    pub fn new(id: u64) -> Self {
        let _trace = profiler::scope("server::pubsub::new");
        Self {
            id,
            channels: AHashSet::new(),
            patterns: AHashSet::new(),
        }
    }

    pub fn subscribe(&mut self, hub: &PubSubHub, channel: &[u8], tx: &UnboundedSender<RespFrame>) {
        let _trace = profiler::scope("server::pubsub::subscribe");
        if self.channels.insert(channel.to_vec()) {
            let _ = hub.subscribe(self.id, channel, tx);
        }
    }

    pub fn unsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> bool {
        let _trace = profiler::scope("server::pubsub::unsubscribe");
        if self.channels.remove(channel) {
            let _ = hub.unsubscribe(self.id, channel);
            true
        } else {
            false
        }
    }

    pub fn psubscribe(&mut self, hub: &PubSubHub, pattern: &[u8], tx: &UnboundedSender<RespFrame>) {
        let _trace = profiler::scope("server::pubsub::psubscribe");
        if self.patterns.insert(pattern.to_vec()) {
            let _ = hub.psubscribe(self.id, pattern, tx);
        }
    }

    pub fn punsubscribe(&mut self, hub: &PubSubHub, pattern: &[u8]) -> bool {
        let _trace = profiler::scope("server::pubsub::punsubscribe");
        if self.patterns.remove(pattern) {
            let _ = hub.punsubscribe(self.id, pattern);
            true
        } else {
            false
        }
    }

    pub fn unsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let _trace = profiler::scope("server::pubsub::unsubscribe_all");
        let channels = self.channels.drain().collect::<Vec<_>>();
        for channel in &channels {
            let _ = hub.unsubscribe(self.id, channel);
        }
        channels
    }

    pub fn punsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let _trace = profiler::scope("server::pubsub::punsubscribe_all");
        let patterns = self.patterns.drain().collect::<Vec<_>>();
        for pattern in &patterns {
            let _ = hub.punsubscribe(self.id, pattern);
        }
        patterns
    }

    pub fn subscription_count(&self) -> i64 {
        let _trace = profiler::scope("server::pubsub::subscription_count");
        (self.channels.len() + self.patterns.len()) as i64
    }
}

fn pattern_prefix(pattern: &[u8]) -> Vec<u8> {
    let _trace = profiler::scope("server::pubsub::pattern_prefix");
    let prefix_len = pattern
        .iter()
        .position(|&byte| byte == b'*' || byte == b'?')
        .unwrap_or(pattern.len());
    pattern[..prefix_len].to_vec()
}

fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
    let _trace = profiler::scope("server::pubsub::wildcard_match");
    let mut pi = 0;
    let mut ti = 0;
    let mut star = None;
    let mut star_match = 0;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == text[ti] || pattern[pi] == b'?') {
            pi += 1;
            ti += 1;
            continue;
        }

        if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            star_match = ti;
            continue;
        }

        match star {
            Some(position) => {
                pi = position + 1;
                star_match += 1;
                ti = star_match;
            }
            None => return false,
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}

fn encode_message_frame(channel: &[u8], payload: &[u8]) -> Bytes {
    let _trace = profiler::scope("server::pubsub::encode_message_frame");
    let frame = RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            b"message",
        )))),
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            channel,
        )))),
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            payload,
        )))),
    ]));
    let mut out = BytesMut::with_capacity(channel.len() + payload.len() + 48);
    encode(&frame, &mut out);
    out.freeze()
}

fn encode_pmessage_frame(pattern: &[u8], channel: &[u8], payload: &[u8]) -> Bytes {
    let _trace = profiler::scope("server::pubsub::encode_pmessage_frame");
    let frame = RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            b"pmessage",
        )))),
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            pattern,
        )))),
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            channel,
        )))),
        RespFrame::Bulk(Some(BulkData::Arg(engine::value::CompactArg::from_slice(
            payload,
        )))),
    ]));
    let mut out = BytesMut::with_capacity(pattern.len() + channel.len() + payload.len() + 56);
    encode(&frame, &mut out);
    out.freeze()
}

fn make_notification_channel(prefix: &[u8], value: &[u8]) -> Vec<u8> {
    let _trace = profiler::scope("server::pubsub::make_notification_channel");
    let mut out = Vec::with_capacity(prefix.len() + value.len());
    out.extend_from_slice(prefix);
    out.extend_from_slice(value);
    out
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
    let _trace = profiler::scope("server::pubsub::flag_to_mask");
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
    let _trace = profiler::scope("server::pubsub::flags_to_mask");
    let mut mask = 0u16;
    for &flag in flags {
        let bit = flag_to_mask(flag).ok_or(())?;
        mask |= bit;
    }
    Ok(mask)
}

fn mask_to_flags(mask: u16) -> Vec<u8> {
    let _trace = profiler::scope("server::pubsub::mask_to_flags");
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
    let _trace = profiler::scope("server::pubsub::notifications_enabled");
    (mask & FLAG_A) != 0 || (flag_to_mask(class).is_some_and(|bit| (mask & bit) != 0))
}

fn notifications_enabled_keyspace(mask: u16) -> bool {
    let _trace = profiler::scope("server::pubsub::notifications_enabled_keyspace");
    (mask & FLAG_KEYSPACE) != 0
}

fn notifications_enabled_keyevent(mask: u16) -> bool {
    let _trace = profiler::scope("server::pubsub::notifications_enabled_keyevent");
    (mask & FLAG_KEYEVENT) != 0
}

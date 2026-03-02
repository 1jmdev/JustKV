use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};

use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::types::{BulkData, RespFrame};

#[derive(Clone)]
pub struct PubSubHub {
    inner: Arc<RwLock<PubSubInner>>,
    next_id: Arc<AtomicU64>,
    notify_mask: Arc<AtomicU16>,
}

struct PubSubInner {
    channels: HashMap<Vec<u8>, HashMap<u64, UnboundedSender<RespFrame>>>,
    patterns: HashMap<Vec<u8>, HashMap<u64, UnboundedSender<RespFrame>>>,
}

impl PubSubHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PubSubInner {
                channels: HashMap::new(),
                patterns: HashMap::new(),
            })),
            next_id: Arc::new(AtomicU64::new(1)),
            notify_mask: Arc::new(AtomicU16::new(0)),
        }
    }

    pub fn next_connection_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn subscribe(&self, id: u64, channel: &[u8], tx: &UnboundedSender<RespFrame>) -> bool {
        let mut inner = self.inner.write();
        let subscribers = inner.channels.entry(channel.to_vec()).or_default();
        subscribers.insert(id, tx.clone()).is_none()
    }

    pub fn unsubscribe(&self, id: u64, channel: &[u8]) -> bool {
        let mut inner = self.inner.write();
        let Some(subscribers) = inner.channels.get_mut(channel) else {
            return false;
        };
        let removed = subscribers.remove(&id).is_some();
        if subscribers.is_empty() {
            inner.channels.remove(channel);
        }
        removed
    }

    pub fn psubscribe(&self, id: u64, pattern: &[u8], tx: &UnboundedSender<RespFrame>) -> bool {
        let mut inner = self.inner.write();
        let subscribers = inner.patterns.entry(pattern.to_vec()).or_default();
        subscribers.insert(id, tx.clone()).is_none()
    }

    pub fn punsubscribe(&self, id: u64, pattern: &[u8]) -> bool {
        let mut inner = self.inner.write();
        let Some(subscribers) = inner.patterns.get_mut(pattern) else {
            return false;
        };
        let removed = subscribers.remove(&id).is_some();
        if subscribers.is_empty() {
            inner.patterns.remove(pattern);
        }
        removed
    }

    pub fn cleanup_connection(&self, id: u64) {
        let mut inner = self.inner.write();
        inner.channels.retain(|_, subscribers| {
            subscribers.remove(&id);
            !subscribers.is_empty()
        });
        inner.patterns.retain(|_, subscribers| {
            subscribers.remove(&id);
            !subscribers.is_empty()
        });
    }

    pub fn publish(&self, channel: &[u8], payload: &[u8]) -> i64 {
        let (channel_subscribers, pattern_subscribers) = {
            let inner = self.inner.read();
            let channel_subscribers = inner
                .channels
                .get(channel)
                .map(|subs| {
                    subs.iter()
                        .map(|(&id, tx)| (id, tx.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let pattern_subscribers = inner
                .patterns
                .iter()
                .filter(|(pattern, _)| wildcard_match(pattern, channel))
                .flat_map(|(pattern, subscribers)| {
                    subscribers
                        .iter()
                        .map(|(&id, tx)| (id, pattern.clone(), tx.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            (channel_subscribers, pattern_subscribers)
        };

        let mut delivered = 0_i64;
        for (id, tx) in channel_subscribers {
            let frame = RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::from_vec(b"message".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(channel.to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(payload.to_vec()))),
            ]));
            if tx.send(frame).is_ok() {
                let _ = id;
                delivered += 1;
            }
        }

        for (id, pattern, tx) in pattern_subscribers {
            let frame = RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::from_vec(b"pmessage".to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(pattern))),
                RespFrame::Bulk(Some(BulkData::from_vec(channel.to_vec()))),
                RespFrame::Bulk(Some(BulkData::from_vec(payload.to_vec()))),
            ]));
            if tx.send(frame).is_ok() {
                let _ = id;
                delivered += 1;
            }
        }

        delivered
    }

    pub fn set_notify_flags(&self, flags: &[u8]) -> Result<(), ()> {
        let mask = flags_to_mask(flags)?;
        self.notify_mask.store(mask, Ordering::Relaxed);
        Ok(())
    }

    pub fn get_notify_flags(&self) -> Vec<u8> {
        mask_to_flags(self.notify_mask.load(Ordering::Relaxed))
    }

    pub fn keyspace_notifications_enabled(&self) -> bool {
        let mask = self.notify_mask.load(Ordering::Relaxed);
        let has_output_target = (mask & (FLAG_KEYSPACE | FLAG_KEYEVENT)) != 0;
        let has_event_classes = (mask
            & (FLAG_A | FLAG_G | FLAG_S | FLAG_H | FLAG_Z | FLAG_L | FLAG_DOLLAR | FLAG_X))
            != 0;
        has_output_target && has_event_classes
    }

    pub fn emit_keyspace_event(&self, event: &[u8], key: &[u8], class: u8) {
        let mask = self.notify_mask.load(Ordering::Relaxed);
        if !notifications_enabled(mask, class) {
            return;
        }

        if notifications_enabled_keyspace(mask) {
            let channel = format!("__keyspace@0__:{}", String::from_utf8_lossy(key));
            let _ = self.publish(channel.as_bytes(), event);
        }
        if notifications_enabled_keyevent(mask) {
            let channel = format!("__keyevent@0__:{}", String::from_utf8_lossy(event));
            let _ = self.publish(channel.as_bytes(), key);
        }
    }

    pub fn pubsub_channels(&self, pattern: Option<&[u8]>) -> Vec<Vec<u8>> {
        let inner = self.inner.read();
        let mut channels = inner
            .channels
            .keys()
            .filter(|channel| pattern.is_none_or(|matcher| wildcard_match(matcher, channel)))
            .cloned()
            .collect::<Vec<_>>();
        channels.sort_unstable();
        channels
    }

    pub fn pubsub_numsub(&self, channels: &[Vec<u8>]) -> Vec<(Vec<u8>, i64)> {
        let inner = self.inner.read();
        channels
            .iter()
            .map(|channel| {
                let count = inner
                    .channels
                    .get(channel.as_slice())
                    .map_or(0_i64, |subscribers| subscribers.len() as i64);
                (channel.clone(), count)
            })
            .collect()
    }

    pub fn pubsub_numpat(&self) -> i64 {
        let inner = self.inner.read();
        inner
            .patterns
            .values()
            .map(|subscribers| subscribers.len() as i64)
            .sum()
    }
}

pub struct ConnectionPubSub {
    pub id: u64,
    channels: HashSet<Vec<u8>>,
    patterns: HashSet<Vec<u8>>,
}

impl ConnectionPubSub {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            channels: HashSet::new(),
            patterns: HashSet::new(),
        }
    }

    pub fn subscribe(&mut self, hub: &PubSubHub, channel: &[u8], tx: &UnboundedSender<RespFrame>) {
        if self.channels.insert(channel.to_vec()) {
            let _ = hub.subscribe(self.id, channel, tx);
        }
    }

    pub fn unsubscribe(&mut self, hub: &PubSubHub, channel: &[u8]) -> bool {
        if self.channels.remove(channel) {
            let _ = hub.unsubscribe(self.id, channel);
            true
        } else {
            false
        }
    }

    pub fn psubscribe(&mut self, hub: &PubSubHub, pattern: &[u8], tx: &UnboundedSender<RespFrame>) {
        if self.patterns.insert(pattern.to_vec()) {
            let _ = hub.psubscribe(self.id, pattern, tx);
        }
    }

    pub fn punsubscribe(&mut self, hub: &PubSubHub, pattern: &[u8]) -> bool {
        if self.patterns.remove(pattern) {
            let _ = hub.punsubscribe(self.id, pattern);
            true
        } else {
            false
        }
    }

    pub fn unsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let channels = self.channels.drain().collect::<Vec<_>>();
        for channel in &channels {
            let _ = hub.unsubscribe(self.id, channel);
        }
        channels
    }

    pub fn punsubscribe_all(&mut self, hub: &PubSubHub) -> Vec<Vec<u8>> {
        let patterns = self.patterns.drain().collect::<Vec<_>>();
        for pattern in &patterns {
            let _ = hub.punsubscribe(self.id, pattern);
        }
        patterns
    }

    pub fn subscription_count(&self) -> i64 {
        (self.channels.len() + self.patterns.len()) as i64
    }
}

fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
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
    let mut mask = 0u16;
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
    (mask & FLAG_A) != 0 || (flag_to_mask(class).is_some_and(|bit| (mask & bit) != 0))
}

fn notifications_enabled_keyspace(mask: u16) -> bool {
    (mask & FLAG_KEYSPACE) != 0
}

fn notifications_enabled_keyevent(mask: u16) -> bool {
    (mask & FLAG_KEYEVENT) != 0
}

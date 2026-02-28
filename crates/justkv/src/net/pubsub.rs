use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::types::{BulkData, RespFrame};

#[derive(Clone)]
pub struct PubSubHub {
    inner: Arc<RwLock<PubSubInner>>,
    next_id: Arc<AtomicU64>,
    notify_flags: Arc<RwLock<Vec<u8>>>,
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
            notify_flags: Arc::new(RwLock::new(b"KEA".to_vec())),
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
        if !flags.iter().all(|flag| is_valid_notify_flag(*flag)) {
            return Err(());
        }
        let mut current = self.notify_flags.write();
        *current = flags.to_vec();
        Ok(())
    }

    pub fn get_notify_flags(&self) -> Vec<u8> {
        self.notify_flags.read().clone()
    }

    pub fn emit_keyspace_event(&self, event: &[u8], key: &[u8], class: u8) {
        let flags = self.notify_flags.read().clone();
        if !notifications_enabled(&flags, class) {
            return;
        }

        if notifications_enabled_keyspace(&flags) {
            let channel = format!("__keyspace@0__:{}", String::from_utf8_lossy(key));
            let _ = self.publish(channel.as_bytes(), event);
        }
        if notifications_enabled_keyevent(&flags) {
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

fn is_valid_notify_flag(flag: u8) -> bool {
    matches!(
        flag,
        b'g' | b'$' | b'l' | b's' | b'h' | b'z' | b'x' | b'e' | b'K' | b'E' | b'A'
    )
}

fn notifications_enabled(flags: &[u8], class: u8) -> bool {
    flags.contains(&b'A') || flags.contains(&class)
}

fn notifications_enabled_keyspace(flags: &[u8]) -> bool {
    flags.contains(&b'A') || flags.contains(&b'K')
}

fn notifications_enabled_keyevent(flags: &[u8]) -> bool {
    flags.contains(&b'A') || flags.contains(&b'E')
}

use ahash::AHashSet;
use protocol::types::RespFrame;
use tokio::sync::mpsc::UnboundedSender;

use super::PubSubHub;

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

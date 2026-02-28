#[derive(Clone, Debug)]
pub struct Entry {
    pub value: Box<[u8]>,
    pub expires_at_ms: u64,
}

impl Entry {
    pub fn new(value: Vec<u8>, expires_at_ms: u64) -> Self {
        Self {
            value: value.into_boxed_slice(),
            expires_at_ms,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.expires_at_ms != 0 && now_ms >= self.expires_at_ms
    }
}

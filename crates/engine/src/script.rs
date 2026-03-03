use crate::store::Store;
use crate::value::{CompactArg, CompactKey, CompactValue};
use sha1::{Digest, Sha1};

impl Store {
    pub fn script_load(&self, script: &[u8]) -> Vec<u8> {
        let _trace = profiler::scope("engine::script::script_load");
        let digest = sha1_hex(script);
        let mut scripts = self.scripts.write();
        scripts.insert(
            CompactKey::from_vec(digest.clone()),
            CompactValue::from_slice(script),
        );
        digest
    }

    pub fn script_get(&self, digest: &[u8]) -> Option<Vec<u8>> {
        let _trace = profiler::scope("engine::script::script_get");
        let scripts = self.scripts.read();
        scripts.get(digest).map(|script| script.to_vec())
    }

    pub fn script_exists(&self, digests: &[CompactArg]) -> Vec<bool> {
        let _trace = profiler::scope("engine::script::script_exists");
        let scripts = self.scripts.read();
        digests
            .iter()
            .map(|digest| scripts.contains_key(digest.as_slice()))
            .collect()
    }

    pub fn script_flush(&self) -> usize {
        let _trace = profiler::scope("engine::script::script_flush");
        let mut scripts = self.scripts.write();
        let removed = scripts.len();
        scripts.clear();
        removed
    }
}

fn sha1_hex(script: &[u8]) -> Vec<u8> {
    let _trace = profiler::scope("engine::script::sha1_hex");
    let digest = Sha1::digest(script);
    let mut out = Vec::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(nibble_to_hex(byte >> 4));
        out.push(nibble_to_hex(byte & 0x0f));
    }
    out
}

#[inline]
fn nibble_to_hex(value: u8) -> u8 {
    if value < 10 {
        b'0' + value
    } else {
        b'a' + (value - 10)
    }
}

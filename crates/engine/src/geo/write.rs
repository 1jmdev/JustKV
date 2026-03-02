use crate::store::Store;
use crate::value::{CompactArg, CompactKey, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::{get_geo_mut, new_geo};

impl Store {
    pub fn geoadd(&self, key: &[u8], items: &[(f64, f64, CompactArg)]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                Entry::Geo(Box::new(new_geo()))
            });
        let geo = get_geo_mut(entry).ok_or(())?;

        let mut added = 0i64;
        for (lon, lat, member) in items {
            if geo
                .insert(CompactKey::from_slice(member.as_slice()), (*lon, *lat))
                .is_none()
            {
                added += 1;
            }
        }
        Ok(added)
    }
}

use crate::store::Store;
use crate::value::CompactArg;

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::get_geo;
use super::math::{geohash11, haversine_meters};

impl Store {
    pub fn geopos(
        &self,
        key: &[u8],
        members: &[CompactArg],
    ) -> Result<Vec<Option<(f64, f64)>>, ()> {
        let _trace = profiler::scope("crates::engine::src::geo::read::geopos");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(vec![None; members.len()]);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(vec![None; members.len()]);
        };
        let geo = get_geo(entry).ok_or(())?;

        Ok(members
            .iter()
            .map(|member| geo.get(member.as_slice()).copied())
            .collect())
    }

    pub fn geodist(&self, key: &[u8], member1: &[u8], member2: &[u8]) -> Result<Option<f64>, ()> {
        let _trace = profiler::scope("crates::engine::src::geo::read::geodist");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let geo = get_geo(entry).ok_or(())?;
        let Some((lon1, lat1)) = geo.get(member1).copied() else {
            return Ok(None);
        };
        let Some((lon2, lat2)) = geo.get(member2).copied() else {
            return Ok(None);
        };

        Ok(Some(haversine_meters(lon1, lat1, lon2, lat2)))
    }

    pub fn geohash(&self, key: &[u8], members: &[CompactArg]) -> Result<Vec<Option<String>>, ()> {
        let _trace = profiler::scope("crates::engine::src::geo::read::geohash");
        let positions = self.geopos(key, members)?;
        Ok(positions
            .into_iter()
            .map(|position| position.map(|(lon, lat)| geohash11(lon, lat)))
            .collect())
    }
}

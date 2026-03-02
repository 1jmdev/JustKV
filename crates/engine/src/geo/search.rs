use crate::store::Store;
use crate::value::{CompactKey, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::math::{haversine_meters, meters_per_lat, meters_per_lon};
use super::{GeoSearchMatch, get_geo, new_geo};

impl Store {
    pub fn geosearch(
        &self,
        key: &[u8],
        center: (f64, f64),
        radius_meters: Option<f64>,
        box_size_meters: Option<(f64, f64)>,
        ascending: bool,
        count: Option<usize>,
    ) -> Result<Vec<GeoSearchMatch>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let geo = get_geo(entry).ok_or(())?;

        let mut out = Vec::new();
        for (member, (lon, lat)) in geo.iter() {
            let dx = meters_per_lon(*lon - center.0, center.1);
            let dy = meters_per_lat(*lat - center.1);
            let inside = if let Some(radius) = radius_meters {
                let distance = haversine_meters(center.0, center.1, *lon, *lat);
                distance <= radius
            } else if let Some((width, height)) = box_size_meters {
                dx.abs() <= width / 2.0 && dy.abs() <= height / 2.0
            } else {
                false
            };

            if inside {
                out.push(GeoSearchMatch {
                    member: member.clone(),
                    longitude: *lon,
                    latitude: *lat,
                    distance_meters: Some(haversine_meters(center.0, center.1, *lon, *lat)),
                });
            }
        }

        out.sort_by(|left, right| {
            let left_d = left.distance_meters.unwrap_or(0.0);
            let right_d = right.distance_meters.unwrap_or(0.0);
            if ascending {
                left_d
                    .total_cmp(&right_d)
                    .then_with(|| left.member.as_slice().cmp(right.member.as_slice()))
            } else {
                right_d
                    .total_cmp(&left_d)
                    .then_with(|| left.member.as_slice().cmp(right.member.as_slice()))
            }
        });

        if let Some(limit) = count {
            out.truncate(limit);
        }
        Ok(out)
    }

    pub fn geosearchstore(
        &self,
        destination: &[u8],
        source: &[u8],
        center: (f64, f64),
        radius_meters: Option<f64>,
        box_size_meters: Option<(f64, f64)>,
        ascending: bool,
        count: Option<usize>,
        store_dist: bool,
    ) -> Result<i64, ()> {
        let matches = self.geosearch(
            source,
            center,
            radius_meters,
            box_size_meters,
            ascending,
            count,
        )?;

        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();

        if matches.is_empty() {
            let _ = shard.remove_key(destination);
            return Ok(0);
        }

        if store_dist {
            let mut zset = crate::value::ZSetValueMap::new();
            for entry in &matches {
                zset.insert(entry.member.clone(), entry.distance_meters.unwrap_or(0.0));
            }
            shard.entries.insert(
                CompactKey::from_slice(destination),
                Entry::ZSet(Box::new(zset)),
            );
        } else {
            let mut geo = new_geo();
            for entry in &matches {
                geo.insert(entry.member.clone(), (entry.longitude, entry.latitude));
            }
            shard.entries.insert(
                CompactKey::from_slice(destination),
                Entry::Geo(Box::new(geo)),
            );
        }
        let _ = shard.clear_ttl(destination);
        Ok(matches.len() as i64)
    }
}

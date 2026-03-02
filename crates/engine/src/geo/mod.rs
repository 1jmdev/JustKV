mod math;
mod read;
mod search;
mod write;

use ahash::RandomState;
use hashbrown::HashMap;

use crate::value::{CompactKey, Entry, GeoValue};

pub struct GeoSearchMatch {
    pub member: CompactKey,
    pub longitude: f64,
    pub latitude: f64,
    pub distance_meters: Option<f64>,
}

fn get_geo(entry: &Entry) -> Option<&GeoValue> {
    entry.as_geo()
}

fn get_geo_mut(entry: &mut Entry) -> Option<&mut GeoValue> {
    entry.as_geo_mut()
}

fn new_geo() -> GeoValue {
    HashMap::with_hasher(RandomState::new())
}

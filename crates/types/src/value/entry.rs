use std::collections::VecDeque;

use rapidhash::fast::RandomState;
use bytes::Bytes;
use hashbrown::HashMap;
use indexmap::IndexSet;
use serde_json::Value as JsonValue;

use super::{CompactKey, CompactValue, StreamValue, ZSetValueMap};

pub type HashValueMap = HashMap<CompactKey, CompactValue, RandomState>;
pub type ListValue = VecDeque<CompactValue>;
pub type SetValue = IndexSet<CompactKey, RandomState>;
pub type GeoValue = HashMap<CompactKey, (f64, f64), RandomState>;

#[derive(Clone, Debug)]
pub struct HashValue {
    map: HashValueMap,
    getall_cache: Option<Bytes>,
}

impl HashValue {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_hasher(RandomState::new()),
            getall_cache: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, RandomState::new()),
            getall_cache: None,
        }
    }

    #[inline]
    pub fn map(&self) -> &HashValueMap {
        &self.map
    }

    #[inline]
    pub fn map_mut(&mut self) -> &mut HashValueMap {
        self.getall_cache = None;
        &mut self.map
    }

    #[inline]
    pub fn getall_cache(&self) -> Option<&Bytes> {
        self.getall_cache.as_ref()
    }

    #[inline]
    pub fn set_getall_cache(&mut self, encoded: Bytes) {
        self.getall_cache = Some(encoded);
    }

    #[inline]
    pub fn invalidate_getall_cache(&mut self) {
        self.getall_cache = None;
    }
}

#[derive(Clone, Debug)]
pub enum Entry {
    String(CompactValue),
    Hash(Box<HashValue>),
    List(Box<ListValue>),
    Set(Box<SetValue>),
    ZSet(Box<ZSetValueMap>),
    Geo(Box<GeoValue>),
    Stream(Box<StreamValue>),
    Json(Box<JsonValue>),
}

impl Entry {
    pub fn from_slice(value: &[u8]) -> Self {
        Self::String(CompactValue::from_slice(value))
    }

    pub fn new(value: Vec<u8>) -> Self {
        Self::String(CompactValue::from_vec(value))
    }

    pub fn empty_hash() -> Self {
        Self::Hash(Box::new(HashValue::new()))
    }

    pub fn as_string(&self) -> Option<&CompactValue> {
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn into_string(self) -> Option<CompactValue> {
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_hash(&self) -> Option<&HashValueMap> {
        match self {
            Self::Hash(value) => Some(value.map()),
            Self::String(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_hash_mut(&mut self) -> Option<&mut HashValueMap> {
        match self {
            Self::Hash(value) => Some(value.map_mut()),
            Self::String(_) => None,
            Self::List(_) => None,
            Self::Set(_) => None,
            Self::ZSet(_) => None,
            Self::Geo(_) => None,
            Self::Stream(_) => None,
            Self::Json(_) => None,
        }
    }

    pub fn as_list(&self) -> Option<&ListValue> {
        match self {
            Self::List(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListValue> {
        match self {
            Self::List(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_set(&self) -> Option<&SetValue> {
        match self {
            Self::Set(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_set_mut(&mut self) -> Option<&mut SetValue> {
        match self {
            Self::Set(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_zset(&self) -> Option<&ZSetValueMap> {
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_zset_mut(&mut self) -> Option<&mut ZSetValueMap> {
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_geo(&self) -> Option<&GeoValue> {
        match self {
            Self::Geo(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_geo_mut(&mut self) -> Option<&mut GeoValue> {
        match self {
            Self::Geo(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_stream(&self) -> Option<&StreamValue> {
        match self {
            Self::Stream(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Json(_) => None,
        }
    }

    pub fn hash_getall_cache(&self) -> Option<&Bytes> {
        match self {
            Self::Hash(value) => value.getall_cache(),
            Self::String(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => None,
        }
    }

    pub fn set_hash_getall_cache(&mut self, encoded: Bytes) -> bool {
        match self {
            Self::Hash(value) => {
                value.set_getall_cache(encoded);
                true
            }
            Self::String(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_)
            | Self::Json(_) => false,
        }
    }

    pub fn invalidate_hash_getall_cache(&mut self) {
        if let Self::Hash(value) = self {
            value.invalidate_getall_cache();
        }
    }

    pub fn as_stream_mut(&mut self) -> Option<&mut StreamValue> {
        match self {
            Self::Stream(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Json(_) => None,
        }
    }

    pub fn as_json(&self) -> Option<&JsonValue> {
        match self {
            Self::Json(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_json_mut(&mut self) -> Option<&mut JsonValue> {
        match self {
            Self::Json(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Hash(_) => "hash",
            Self::List(_) => "list",
            Self::Set(_) => "set",
            Self::ZSet(_) => "zset",
            Self::Geo(_) => "zset",
            Self::Stream(_) => "stream",
            Self::Json(_) => "json",
        }
    }
}

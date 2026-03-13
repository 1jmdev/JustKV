use std::collections::VecDeque;

use bytes::Bytes;
use hashbrown::HashMap;
use indexmap::IndexSet;
use rapidhash::fast::RandomState;
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

impl Default for HashValue {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! entry_ref_accessor {
    ($name:ident, $variant:ident, $ty:ty) => {
        pub fn $name(&self) -> Option<&$ty> {
            if let Self::$variant(value) = self {
                Some(value)
            } else {
                None
            }
        }
    };
}

macro_rules! entry_mut_accessor {
    ($name:ident, $variant:ident, $ty:ty) => {
        pub fn $name(&mut self) -> Option<&mut $ty> {
            if let Self::$variant(value) = self {
                Some(value)
            } else {
                None
            }
        }
    };
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
        Self::Hash(Box::default())
    }

    entry_ref_accessor!(as_string, String, CompactValue);

    pub fn into_string(self) -> Option<CompactValue> {
        if let Self::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_hash(&self) -> Option<&HashValueMap> {
        if let Self::Hash(value) = self {
            Some(value.map())
        } else {
            None
        }
    }

    pub fn as_hash_mut(&mut self) -> Option<&mut HashValueMap> {
        if let Self::Hash(value) = self {
            Some(value.map_mut())
        } else {
            None
        }
    }

    entry_ref_accessor!(as_list, List, ListValue);
    entry_mut_accessor!(as_list_mut, List, ListValue);

    entry_ref_accessor!(as_set, Set, SetValue);
    entry_mut_accessor!(as_set_mut, Set, SetValue);

    entry_ref_accessor!(as_zset, ZSet, ZSetValueMap);
    entry_mut_accessor!(as_zset_mut, ZSet, ZSetValueMap);

    entry_ref_accessor!(as_geo, Geo, GeoValue);
    entry_mut_accessor!(as_geo_mut, Geo, GeoValue);

    entry_ref_accessor!(as_stream, Stream, StreamValue);
    entry_mut_accessor!(as_stream_mut, Stream, StreamValue);

    pub fn hash_getall_cache(&self) -> Option<&Bytes> {
        if let Self::Hash(value) = self {
            value.getall_cache()
        } else {
            None
        }
    }

    pub fn set_hash_getall_cache(&mut self, encoded: Bytes) -> bool {
        if let Self::Hash(value) = self {
            value.set_getall_cache(encoded);
            true
        } else {
            false
        }
    }

    pub fn invalidate_hash_getall_cache(&mut self) {
        if let Self::Hash(value) = self {
            value.invalidate_getall_cache();
        }
    }

    entry_ref_accessor!(as_json, Json, JsonValue);
    entry_mut_accessor!(as_json_mut, Json, JsonValue);

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

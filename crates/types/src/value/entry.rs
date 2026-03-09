use std::collections::VecDeque;

use ahash::RandomState;
use hashbrown::HashMap;
use indexmap::IndexSet;
use serde_json::Value as JsonValue;

use super::{CompactKey, CompactValue, StreamValue, ZSetValueMap};

pub type HashValueMap = HashMap<CompactKey, CompactValue, RandomState>;
pub type ListValue = VecDeque<CompactValue>;
pub type SetValue = IndexSet<CompactKey, RandomState>;
pub type GeoValue = HashMap<CompactKey, (f64, f64), RandomState>;

#[derive(Clone, Debug)]
pub enum Entry {
    String(CompactValue),
    Hash(Box<HashValueMap>),
    List(Box<ListValue>),
    Set(Box<SetValue>),
    ZSet(Box<ZSetValueMap>),
    Geo(Box<GeoValue>),
    Stream(Box<StreamValue>),
    Json(Box<JsonValue>),
}

impl Entry {
    pub fn from_slice(value: &[u8]) -> Self {
        let _trace = profiler::scope("crates::types::src::value::from_slice");
        Self::String(CompactValue::from_slice(value))
    }

    pub fn new(value: Vec<u8>) -> Self {
        let _trace = profiler::scope("crates::types::src::value::new");
        Self::String(CompactValue::from_vec(value))
    }

    pub fn empty_hash() -> Self {
        let _trace = profiler::scope("crates::types::src::value::empty_hash");
        Self::Hash(Box::new(HashMap::with_hasher(RandomState::new())))
    }

    pub fn as_string(&self) -> Option<&CompactValue> {
        let _trace = profiler::scope("crates::types::src::value::as_string");
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
        let _trace = profiler::scope("crates::types::src::value::into_string");
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
        let _trace = profiler::scope("crates::types::src::value::as_hash");
        match self {
            Self::Hash(value) => Some(value),
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
        let _trace = profiler::scope("crates::types::src::value::as_hash_mut");
        match self {
            Self::Hash(value) => Some(value),
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
        let _trace = profiler::scope("crates::types::src::value::as_list");
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
        let _trace = profiler::scope("crates::types::src::value::as_list_mut");
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
        let _trace = profiler::scope("crates::types::src::value::as_set");
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
        let _trace = profiler::scope("crates::types::src::value::as_set_mut");
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
        let _trace = profiler::scope("crates::types::src::value::as_zset");
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
        let _trace = profiler::scope("crates::types::src::value::as_zset_mut");
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
        let _trace = profiler::scope("crates::types::src::value::as_geo");
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
        let _trace = profiler::scope("crates::types::src::value::as_geo_mut");
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
        let _trace = profiler::scope("crates::types::src::value::as_stream");
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

    pub fn as_stream_mut(&mut self) -> Option<&mut StreamValue> {
        let _trace = profiler::scope("crates::types::src::value::as_stream_mut");
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
        let _trace = profiler::scope("crates::types::src::value::as_json");
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
        let _trace = profiler::scope("crates::types::src::value::as_json_mut");
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
        let _trace = profiler::scope("crates::types::src::value::kind");
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

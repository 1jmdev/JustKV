#[cfg(feature = "store-hashmap")]
mod hashbrown_map;

#[cfg(not(feature = "store-hashmap"))]
mod constants;
#[cfg(not(feature = "store-hashmap"))]
mod index;
#[cfg(not(feature = "store-hashmap"))]
mod insert_ops;
#[cfg(not(feature = "store-hashmap"))]
mod iter;
#[cfg(not(feature = "store-hashmap"))]
mod lookup_ops;
#[cfg(not(feature = "store-hashmap"))]
mod node;
#[cfg(not(feature = "store-hashmap"))]
mod rehash_ops;
#[cfg(not(feature = "store-hashmap"))]
mod remove_ops;
#[cfg(not(feature = "store-hashmap"))]
mod table;
#[cfg(not(feature = "store-hashmap"))]
mod types;

#[cfg(feature = "store-hashmap")]
pub(super) use hashbrown_map::RehashingMap;
#[cfg(not(feature = "store-hashmap"))]
pub(super) use types::RehashingMap;

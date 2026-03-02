use crate::engine::store::{ListSide, Store};
use crate::engine::value::{CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_list_mut;

impl Store {
    pub fn lmove(
        &self,
        source: &[u8],
        destination: &[u8],
        from: ListSide,
        to: ListSide,
    ) -> Result<Option<CompactValue>, ()> {
        let source_idx = self.shard_index(source);
        let destination_idx = self.shard_index(destination);
        let now_ms = monotonic_now_ms();

        if source_idx == destination_idx {
            let mut shard = self.shards[source_idx].write();
            let _ = purge_if_expired(&mut shard, source, now_ms);
            if source != destination {
                let _ = purge_if_expired(&mut shard, destination, now_ms);
            }
            return move_inside_shard(&mut shard, source, destination, from, to);
        }

        let (first_idx, second_idx, source_is_first) = if source_idx < destination_idx {
            (source_idx, destination_idx, true)
        } else {
            (destination_idx, source_idx, false)
        };

        let mut first = self.shards[first_idx].write();
        let mut second = self.shards[second_idx].write();

        if source_is_first {
            let _ = purge_if_expired(&mut first, source, now_ms);
            let _ = purge_if_expired(&mut second, destination, now_ms);
            move_across_shards(&mut first, source, &mut second, destination, from, to)
        } else {
            let _ = purge_if_expired(&mut second, source, now_ms);
            let _ = purge_if_expired(&mut first, destination, now_ms);
            move_across_shards(&mut second, source, &mut first, destination, from, to)
        }
    }

    pub fn rpoplpush(&self, source: &[u8], destination: &[u8]) -> Result<Option<CompactValue>, ()> {
        self.lmove(source, destination, ListSide::Right, ListSide::Left)
    }
}

fn move_inside_shard(
    shard: &mut super::super::Shard,
    source: &[u8],
    destination: &[u8],
    from: ListSide,
    to: ListSide,
) -> Result<Option<CompactValue>, ()> {
    let Some(entry) = shard.entries.get_mut(source) else {
        return Ok(None);
    };
    let list = get_list_mut(entry).ok_or(())?;
    let moved = pop_side(list, from);
    if moved.is_none() {
        return Ok(None);
    }

    if source == destination {
        if let Some(value) = moved.clone() {
            push_side(list, value, to);
        }
        return Ok(moved);
    }

    let source_empty = list.is_empty();
    if source_empty {
        let _ = shard.remove_key(source);
    }

    let destination_entry = shard
        .entries
        .get_or_insert_with(CompactKey::from_slice(destination), || {
            Entry::List(Box::new(std::collections::VecDeque::new()))
        });
    let destination_list = get_list_mut(destination_entry).ok_or(())?;
    if let Some(value) = moved.clone() {
        push_side(destination_list, value, to);
    }
    Ok(moved)
}

fn move_across_shards(
    source_shard: &mut super::super::Shard,
    source: &[u8],
    destination_shard: &mut super::super::Shard,
    destination: &[u8],
    from: ListSide,
    to: ListSide,
) -> Result<Option<CompactValue>, ()> {
    let Some(entry) = source_shard.entries.get_mut(source) else {
        return Ok(None);
    };
    let source_list = get_list_mut(entry).ok_or(())?;
    let moved = pop_side(source_list, from);
    if moved.is_none() {
        return Ok(None);
    }

    if source_list.is_empty() {
        let _ = source_shard.remove_key(source);
    }

    let destination_entry = destination_shard
        .entries
        .get_or_insert_with(CompactKey::from_slice(destination), || {
            Entry::List(Box::new(std::collections::VecDeque::new()))
        });
    let destination_list = get_list_mut(destination_entry).ok_or(())?;
    if let Some(value) = moved.clone() {
        push_side(destination_list, value, to);
    }

    Ok(moved)
}

fn pop_side(
    list: &mut std::collections::VecDeque<CompactValue>,
    side: ListSide,
) -> Option<CompactValue> {
    match side {
        ListSide::Left => list.pop_front(),
        ListSide::Right => list.pop_back(),
    }
}

fn push_side(
    list: &mut std::collections::VecDeque<CompactValue>,
    value: CompactValue,
    side: ListSide,
) {
    match side {
        ListSide::Left => list.push_front(value),
        ListSide::Right => list.push_back(value),
    }
}

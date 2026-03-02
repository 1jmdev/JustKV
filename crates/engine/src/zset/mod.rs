mod algebra;
mod core;
mod pop;
mod random;
mod range;
mod scan;

use crate::value::{CompactKey, Entry, ZSetValueMap};

fn get_zset(entry: &Entry) -> Option<&ZSetValueMap> {
    entry.as_zset()
}

fn get_zset_mut(entry: &mut Entry) -> Option<&mut ZSetValueMap> {
    entry.as_zset_mut()
}

fn sorted_by_score(map: &ZSetValueMap, reverse: bool) -> Vec<(CompactKey, f64)> {
    map.iter_ordered(reverse)
        .map(|(member, score)| (member.clone(), score))
        .collect()
}

fn sorted_by_score_refs(map: &ZSetValueMap, reverse: bool) -> Vec<(&CompactKey, f64)> {
    map.iter_ordered(reverse).collect()
}

fn compare_member_score(
    left: &(CompactKey, f64),
    right: &(CompactKey, f64),
    reverse: bool,
) -> std::cmp::Ordering {
    let score_order = left.1.total_cmp(&right.1);
    let score_order = if reverse {
        score_order.reverse()
    } else {
        score_order
    };
    if score_order == std::cmp::Ordering::Equal {
        if reverse {
            right.0.as_slice().cmp(left.0.as_slice())
        } else {
            left.0.as_slice().cmp(right.0.as_slice())
        }
    } else {
        score_order
    }
}

fn normalize_range(start: i64, stop: i64, len: usize) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }
    let len_i64 = len as i64;
    let mut from = if start < 0 { len_i64 + start } else { start };
    let mut to = if stop < 0 { len_i64 + stop } else { stop };

    if from < 0 {
        from = 0;
    }
    if to < 0 {
        return None;
    }
    if from >= len_i64 {
        return None;
    }
    if to >= len_i64 {
        to = len_i64 - 1;
    }
    if from > to {
        return None;
    }
    Some((from as usize, (to as usize) + 1))
}

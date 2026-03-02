use ahash::RandomState;
use hashbrown::HashMap;

use crate::engine::store::{XAddId, XTrimMode};
use crate::engine::value::{
    CompactArg, CompactKey, CompactValue, StreamId, StreamPendingEntry, StreamValue,
};

pub struct StreamRangeItem {
    pub id: StreamId,
    pub fields: Vec<(CompactKey, CompactValue)>,
}

pub struct XPendingSummary {
    pub total: i64,
    pub min: Option<StreamId>,
    pub max: Option<StreamId>,
    pub consumers: HashMap<CompactKey, i64, RandomState>,
}

pub(super) fn xadd_into_stream(
    stream: &mut StreamValue,
    id: XAddId,
    fields: &[(CompactArg, CompactArg)],
    trim: Option<(XTrimMode, StreamId, Option<usize>)>,
) -> Result<Option<StreamId>, ()> {
    let assigned = match id {
        XAddId::Auto => {
            let now = current_unix_ms();
            if now > stream.last_id.ms {
                StreamId { ms: now, seq: 0 }
            } else {
                StreamId {
                    ms: stream.last_id.ms,
                    seq: stream.last_id.seq.saturating_add(1),
                }
            }
        }
        XAddId::Explicit { ms, seq } => StreamId { ms, seq },
        XAddId::AutoSeqAtMs { ms } => {
            if ms > stream.last_id.ms {
                StreamId { ms, seq: 0 }
            } else if ms == stream.last_id.ms {
                StreamId {
                    ms,
                    seq: stream.last_id.seq.saturating_add(1),
                }
            } else {
                return Ok(None);
            }
        }
    };

    if assigned <= stream.last_id {
        return Ok(None);
    }

    stream.entries.insert(
        assigned,
        fields
            .iter()
            .map(|(field, value)| {
                (
                    CompactKey::from_slice(field.as_slice()),
                    CompactValue::from_slice(value.as_slice()),
                )
            })
            .collect(),
    );
    stream.last_id = assigned;

    if let Some((mode, threshold, limit)) = trim {
        apply_trim(stream, mode, threshold, limit);
    }
    Ok(Some(assigned))
}

pub(super) fn apply_trim(
    stream: &mut StreamValue,
    mode: XTrimMode,
    threshold: StreamId,
    limit: Option<usize>,
) {
    let ids: Vec<StreamId> = match mode {
        XTrimMode::MaxLen => {
            let max_len = threshold.ms as usize;
            if stream.entries.len() <= max_len {
                Vec::new()
            } else {
                let drop = stream.entries.len() - max_len;
                stream.entries.keys().copied().take(drop).collect()
            }
        }
        XTrimMode::MinId => stream
            .entries
            .keys()
            .copied()
            .take_while(|id| *id < threshold)
            .collect(),
    };

    let mut removed = 0usize;
    for id in ids {
        if let Some(max) = limit && removed >= max {
            break;
        }
        if stream.entries.remove(&id).is_some() {
            removed += 1;
        }
        for group in stream.groups.values_mut() {
            let _ = group.pending.remove(&id);
        }
    }
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn push_items(
    stream: &StreamValue,
    start: StreamId,
    end: StreamId,
    reverse: bool,
    count: Option<usize>,
) -> Vec<StreamRangeItem> {
    let iter = stream.entries.range(start..=end);
    let mut out: Vec<_> = if reverse {
        iter.rev()
            .map(|(id, fields)| StreamRangeItem {
                id: *id,
                fields: fields.clone(),
            })
            .collect()
    } else {
        iter.map(|(id, fields)| StreamRangeItem {
            id: *id,
            fields: fields.clone(),
        })
        .collect()
    };
    if let Some(limit) = count {
        out.truncate(limit);
    }
    out
}

pub(super) fn ensure_pending_entry(
    pending: &mut HashMap<StreamId, StreamPendingEntry, RandomState>,
    id: StreamId,
    consumer: &[u8],
) {
    let value = pending.entry(id).or_insert(StreamPendingEntry {
        consumer: CompactKey::from_slice(consumer),
        deliveries: 0,
    });
    value.consumer = CompactKey::from_slice(consumer);
    value.deliveries = value.deliveries.saturating_add(1);
}

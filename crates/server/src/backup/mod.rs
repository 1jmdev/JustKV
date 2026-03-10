mod io;
mod rdb;

use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ahash::RandomState;
use hashbrown::HashMap;
use indexmap::IndexSet;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

use crate::config::SnapshotCompression;
use engine::store::{PreDecodedRestoreEntry, Store};
use types::value::{
    CompactKey, CompactValue, Entry as StoreEntry, HashValue, StreamId, StreamValue, ZSetValueMap,
};

use self::rdb::{
    CountingWriter, EMBEDDED_PREFIX, OP_EOF, OP_EXPIRETIME, OP_SELECTDB, RDB_MAGIC_PREFIX,
    RDB_VERSION, RdbValue, Value, decode_custom_entry, encode_len, parse_rdb, write_rdb_value,
};

#[derive(Debug, Clone, Copy)]
pub struct SnapshotStats {
    pub keys_written: u64,
    pub uncompressed_bytes: u64,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct RestoreStats {
    pub keys_loaded: u64,
}

pub async fn load_snapshot(store: &Store, path: &Path) -> Result<RestoreStats, String> {
    let _trace = profiler::scope("server::backup::load_snapshot");
    let bytes = decode_snapshot_file(&io::read_snapshot_bytes(path).await?)?;
    let entries = parse_rdb(&bytes)?;

    let now_s = now_unix_seconds();
    let mut loaded = 0u64;
    let mut restore_entries = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(expire_at) = entry.expire_at_s
            && u64::from(expire_at) <= now_s
        {
            continue;
        }

        let ttl_ms = entry
            .expire_at_s
            .map(|ts| {
                let remaining_s = u64::from(ts).saturating_sub(now_s);
                remaining_s.saturating_mul(1000)
            })
            .unwrap_or(0);

        restore_entries.push(PreDecodedRestoreEntry {
            key: entry.key,
            ttl_ms,
            entry: value_into_store_entry(entry.value),
        });
        loaded += 1;
    }

    store.restore_predecoded_unchecked(restore_entries);

    Ok(RestoreStats {
        keys_loaded: loaded,
    })
}

pub fn write_snapshot(
    store: &Store,
    path: &Path,
    compression: SnapshotCompression,
) -> Result<SnapshotStats, String> {
    let _trace = profiler::scope("server::backup::write_snapshot");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create snapshot directory {}: {err}",
                parent.display()
            )
        })?;
    }

    let (written_keys, uncompressed) =
        store.with_transaction_gate(|| build_rdb_snapshot(store, path))?;
    let encoded = encode_snapshot_file(&uncompressed, compression);
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, &encoded)
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;
    io::sync_file_path(&temp_path)?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        format!(
            "failed to move snapshot {} to {}: {err}",
            temp_path.display(),
            path.display()
        )
    })?;
    io::sync_directory(path)?;

    Ok(SnapshotStats {
        keys_written: written_keys,
        uncompressed_bytes: uncompressed.len() as u64,
        bytes_written: encoded.len() as u64,
    })
}

fn build_rdb_snapshot(store: &Store, path: &Path) -> Result<(u64, Vec<u8>), String> {
    let _trace = profiler::scope("server::backup::build_rdb_snapshot");
    let mut writer = CountingWriter::new(Vec::with_capacity(256 * 1024));

    writer
        .write_all(RDB_MAGIC_PREFIX)
        .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
    writer
        .write_all(RDB_VERSION.as_bytes())
        .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
    writer
        .write_all(&[OP_SELECTDB])
        .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
    encode_len(&mut writer, 0)
        .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;

    let now_s = now_unix_seconds();
    let mut written_keys = 0u64;
    let mut cursor = 0u64;
    loop {
        let (next_cursor, keys) = store.scan(cursor, None, 8_192, None);
        for key in keys {
            let key_bytes = key.as_slice();
            let Some(payload) = store.dump(key_bytes) else {
                continue;
            };

            let ttl_ms = store.pttl(key_bytes);
            let value = decode_custom_entry(&payload)?;
            let rdb_value = match value {
                Value::Geo(_) | Value::Stream(_) => {
                    let mut bytes = EMBEDDED_PREFIX.to_vec();
                    bytes.extend_from_slice(&payload);
                    RdbValue::String(bytes)
                }
                Value::String(value) => RdbValue::String(value),
                Value::Hash(pairs) => RdbValue::Hash(pairs),
                Value::List(values) => RdbValue::List(values),
                Value::Set(values) => RdbValue::Set(values),
                Value::ZSet(values) => RdbValue::ZSet(values),
            };

            if ttl_ms >= 0 {
                let expire_at_s = now_s.saturating_add((ttl_ms as u64).div_ceil(1000));
                let expire_at_s_u32 = u32::try_from(expire_at_s)
                    .map_err(|_| format!("ttl overflow while writing {}", path.display()))?;
                writer
                    .write_all(&[OP_EXPIRETIME])
                    .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
                writer
                    .write_all(&expire_at_s_u32.to_le_bytes())
                    .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
            }

            write_rdb_value(&mut writer, key_bytes, rdb_value)
                .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
            written_keys = written_keys.saturating_add(1);
        }

        if next_cursor == 0 {
            break;
        }
        cursor = next_cursor;
    }

    writer
        .write_all(&[OP_EOF])
        .map_err(|err| format!("failed to write snapshot {}: {err}", path.display()))?;
    Ok((written_keys, writer.into_inner()))
}

const SNAPSHOT_MAGIC: &[u8; 8] = b"BKVSNP01";
const SNAPSHOT_CODEC_NONE: u8 = 0;
const SNAPSHOT_CODEC_LZ4: u8 = 1;

fn encode_snapshot_file(raw_rdb: &[u8], compression: SnapshotCompression) -> Vec<u8> {
    let _trace = profiler::scope("server::backup::encode_snapshot_file");
    match compression {
        SnapshotCompression::None => raw_rdb.to_vec(),
        SnapshotCompression::Lz4 => {
            let compressed = compress_prepend_size(raw_rdb);
            let mut out = Vec::with_capacity(SNAPSHOT_MAGIC.len() + 1 + compressed.len());
            out.extend_from_slice(SNAPSHOT_MAGIC);
            out.push(SNAPSHOT_CODEC_LZ4);
            out.extend_from_slice(&compressed);
            out
        }
    }
}

fn decode_snapshot_file(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::decode_snapshot_file");
    if bytes.starts_with(RDB_MAGIC_PREFIX) {
        return Ok(bytes.to_vec());
    }
    if bytes.len() < SNAPSHOT_MAGIC.len() + 1 || &bytes[..SNAPSHOT_MAGIC.len()] != SNAPSHOT_MAGIC {
        return Err("snapshot has unknown header".to_string());
    }

    match bytes[SNAPSHOT_MAGIC.len()] {
        SNAPSHOT_CODEC_NONE => Ok(bytes[SNAPSHOT_MAGIC.len() + 1..].to_vec()),
        SNAPSHOT_CODEC_LZ4 => decompress_size_prepended(&bytes[SNAPSHOT_MAGIC.len() + 1..])
            .map_err(|err| format!("failed to decompress snapshot: {err}")),
        other => Err(format!("unsupported snapshot codec {other}")),
    }
}

fn value_into_store_entry(value: Value) -> StoreEntry {
    let _trace = profiler::scope("server::backup::value_into_store_entry");
    match value {
        Value::String(v) => StoreEntry::String(CompactValue::from_vec(v)),
        Value::Hash(values) => {
            let mut hash_value = HashValue::with_capacity(values.len());
            let map = hash_value.map_mut();
            for (field, value) in values {
                map.insert(CompactKey::from_vec(field), CompactValue::from_vec(value));
            }
            StoreEntry::Hash(Box::new(hash_value))
        }
        Value::List(values) => {
            let mut list = VecDeque::with_capacity(values.len());
            for value in values {
                list.push_back(CompactValue::from_vec(value));
            }
            StoreEntry::List(Box::new(list))
        }
        Value::Set(values) => {
            let mut set: IndexSet<CompactKey, RandomState> =
                IndexSet::with_capacity_and_hasher(values.len(), RandomState::new());
            for value in values {
                set.insert(CompactKey::from_vec(value));
            }
            StoreEntry::Set(Box::new(set))
        }
        Value::ZSet(values) => {
            let mut zset = ZSetValueMap::new();
            for (member, score) in values {
                zset.insert(CompactKey::from_vec(member), score);
            }
            StoreEntry::ZSet(Box::new(zset))
        }
        Value::Geo(values) => {
            let mut geo: HashMap<CompactKey, (f64, f64), RandomState> =
                HashMap::with_capacity_and_hasher(values.len(), RandomState::new());
            for (member, coords) in values {
                geo.insert(CompactKey::from_vec(member), coords);
            }
            StoreEntry::Geo(Box::new(geo))
        }
        Value::Stream(values) => {
            let mut stream = StreamValue::new();
            for entry in values {
                let id = StreamId {
                    ms: entry.ms,
                    seq: entry.seq,
                };
                let mut fields = Vec::with_capacity(entry.fields.len());
                for (field, value) in entry.fields {
                    fields.push((CompactKey::from_vec(field), CompactValue::from_vec(value)));
                }
                stream.last_id = id;
                stream.entries.insert(id, fields);
            }
            StoreEntry::Stream(Box::new(stream))
        }
    }
}

fn now_unix_seconds() -> u64 {
    let _trace = profiler::scope("server::backup::now_unix_seconds");
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ahash::RandomState;
use hashbrown::HashMap;
use indexmap::IndexSet;

use engine::store::{PreDecodedRestoreEntry, Store};
use engine::value::{
    CompactKey, CompactValue, Entry as StoreEntry, StreamId, StreamValue, ZSetValueMap,
};

const RDB_MAGIC_PREFIX: &[u8; 5] = b"REDIS";
const RDB_VERSION: &str = "0004";
const OP_EXPIRETIME: u8 = 0xFD;
const OP_SELECTDB: u8 = 0xFE;
const OP_EOF: u8 = 0xFF;
const TYPE_STRING: u8 = 0;
const TYPE_LIST: u8 = 1;
const TYPE_SET: u8 = 2;
const TYPE_ZSET: u8 = 3;
const TYPE_HASH: u8 = 4;
const EMBEDDED_PREFIX: &[u8] = b"__BETTERKV_EMBEDDED__";

#[derive(Debug, Clone, Copy)]
pub struct SnapshotStats {
    pub keys_written: u64,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct RestoreStats {
    pub keys_loaded: u64,
}

#[derive(Debug, Clone)]
enum Value {
    String(Vec<u8>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    ZSet(Vec<(Vec<u8>, f64)>),
    Geo(Vec<(Vec<u8>, (f64, f64))>),
    Stream(Vec<StreamEntry>),
}

#[derive(Debug, Clone)]
struct StreamEntry {
    ms: u64,
    seq: u64,
    fields: Vec<(Vec<u8>, Vec<u8>)>,
}

#[derive(Debug)]
struct LoadedEntry {
    key: Vec<u8>,
    expire_at_s: Option<u32>,
    value: Value,
}

pub async fn load_snapshot(store: &Store, path: &Path) -> Result<RestoreStats, String> {
    let _trace = profiler::scope("server::backup::load_snapshot");
    let bytes = read_snapshot_bytes(path).await?;
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

fn value_into_store_entry(value: Value) -> StoreEntry {
    let _trace = profiler::scope("server::backup::value_into_store_entry");
    match value {
        Value::String(v) => StoreEntry::String(CompactValue::from_vec(v)),
        Value::Hash(values) => {
            let mut map: HashMap<CompactKey, CompactValue, RandomState> =
                HashMap::with_capacity_and_hasher(values.len(), RandomState::new());
            for (field, value) in values {
                map.insert(CompactKey::from_vec(field), CompactValue::from_vec(value));
            }
            StoreEntry::Hash(Box::new(map))
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

#[cfg(target_os = "linux")]
async fn read_snapshot_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes");
    match read_snapshot_bytes_io_uring(path).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            tracing::warn!(
                error = %err,
                path = %path.display(),
                "io_uring snapshot read failed, falling back to std::fs::read"
            );
            std::fs::read(path).map_err(|read_err| {
                format!("failed to read snapshot {}: {read_err}", path.display())
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
async fn read_snapshot_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes");
    std::fs::read(path).map_err(|err| format!("failed to read snapshot {}: {err}", path.display()))
}

#[cfg(target_os = "linux")]
async fn read_snapshot_bytes_io_uring(path: &Path) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_snapshot_bytes_io_uring");
    let path_buf = path.to_path_buf();
    let path_for_runtime = path_buf.clone();
    let file_size = file_size_bytes(&path_buf)?;

    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move { read_with_io_uring(path_for_runtime, file_size).await })
    })
    .await
    .map_err(|err| {
        format!(
            "failed to join io_uring reader task for {}: {err}",
            path.display()
        )
    })?
}

#[cfg(target_os = "linux")]
async fn read_with_io_uring(path: PathBuf, file_size: usize) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_with_io_uring");
    let file = tokio_uring::fs::File::open(path.clone())
        .await
        .map_err(|err| format!("failed to open snapshot {}: {err}", path.display()))?;

    let mut bytes = Vec::with_capacity(file_size);
    let mut offset = 0u64;
    const CHUNK_SIZE: usize = 4 * 1024 * 1024;

    while bytes.len() < file_size {
        let remaining = file_size - bytes.len();
        let read_len = remaining.min(CHUNK_SIZE);
        let read_buf = vec![0u8; read_len];
        let (result, read_buf) = file.read_at(read_buf, offset).await;
        let read = result
            .map_err(|err| format!("failed to read snapshot chunk {}: {err}", path.display()))?;
        if read == 0 {
            return Err(format!(
                "snapshot {} ended early: expected {file_size} bytes, got {}",
                path.display(),
                bytes.len()
            ));
        }
        bytes.extend_from_slice(&read_buf[..read]);
        offset = offset.saturating_add(read as u64);
    }

    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn file_size_bytes(path: &Path) -> Result<usize, String> {
    let _trace = profiler::scope("server::backup::file_size_bytes");
    let len = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat snapshot {}: {err}", path.display()))?
        .len();
    usize::try_from(len).map_err(|_| {
        format!(
            "snapshot {} is too large to fit in memory on this platform",
            path.display()
        )
    })
}

pub fn write_snapshot(store: &Store, path: &Path) -> Result<SnapshotStats, String> {
    let _trace = profiler::scope("server::backup::write_snapshot");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create snapshot directory {}: {err}",
                parent.display()
            )
        })?;
    }

    let temp_path = path.with_extension("tmp");
    let file = File::create(&temp_path)
        .map_err(|err| format!("failed to create snapshot {}: {err}", temp_path.display()))?;
    let mut writer = CountingWriter::new(BufWriter::new(file));

    writer
        .write_all(RDB_MAGIC_PREFIX)
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;
    writer
        .write_all(RDB_VERSION.as_bytes())
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;

    writer
        .write_all(&[OP_SELECTDB])
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;
    encode_len(&mut writer, 0)
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;

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
                writer.write_all(&[OP_EXPIRETIME]).map_err(|err| {
                    format!("failed to write snapshot {}: {err}", temp_path.display())
                })?;
                writer
                    .write_all(&expire_at_s_u32.to_le_bytes())
                    .map_err(|err| {
                        format!("failed to write snapshot {}: {err}", temp_path.display())
                    })?;
            }

            write_rdb_value(&mut writer, key_bytes, rdb_value).map_err(|err| {
                format!("failed to write snapshot {}: {err}", temp_path.display())
            })?;
            written_keys += 1;
        }

        if next_cursor == 0 {
            break;
        }
        cursor = next_cursor;
    }

    writer
        .write_all(&[OP_EOF])
        .map_err(|err| format!("failed to write snapshot {}: {err}", temp_path.display()))?;
    writer
        .flush()
        .map_err(|err| format!("failed to flush snapshot {}: {err}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        format!(
            "failed to move snapshot {} to {}: {err}",
            temp_path.display(),
            path.display()
        )
    })?;

    Ok(SnapshotStats {
        keys_written: written_keys,
        bytes_written: writer.bytes_written(),
    })
}

enum RdbValue {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    ZSet(Vec<(Vec<u8>, f64)>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
}

fn write_rdb_value<W: Write>(out: &mut W, key: &[u8], value: RdbValue) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::write_rdb_value");
    match value {
        RdbValue::String(v) => {
            out.write_all(&[TYPE_STRING])
                .map_err(|err| err.to_string())?;
            encode_string(out, key)?;
            encode_string(out, &v)?;
        }
        RdbValue::List(values) => {
            out.write_all(&[TYPE_LIST]).map_err(|err| err.to_string())?;
            encode_string(out, key)?;
            encode_len(out, values.len())?;
            for value in values {
                encode_string(out, &value)?;
            }
        }
        RdbValue::Set(values) => {
            out.write_all(&[TYPE_SET]).map_err(|err| err.to_string())?;
            encode_string(out, key)?;
            encode_len(out, values.len())?;
            for value in values {
                encode_string(out, &value)?;
            }
        }
        RdbValue::ZSet(values) => {
            out.write_all(&[TYPE_ZSET]).map_err(|err| err.to_string())?;
            encode_string(out, key)?;
            encode_len(out, values.len())?;
            for (member, score) in values {
                encode_string(out, &member)?;
                encode_zset_score(out, score)?;
            }
        }
        RdbValue::Hash(values) => {
            out.write_all(&[TYPE_HASH]).map_err(|err| err.to_string())?;
            encode_string(out, key)?;
            encode_len(out, values.len())?;
            for (field, value) in values {
                encode_string(out, &field)?;
                encode_string(out, &value)?;
            }
        }
    }

    Ok(())
}

struct CountingWriter<W> {
    inner: W,
    bytes_written: u64,
}

impl<W> CountingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }

    fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes_written = self.bytes_written.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn parse_rdb(input: &[u8]) -> Result<Vec<LoadedEntry>, String> {
    let _trace = profiler::scope("server::backup::parse_rdb");
    if input.len() < 9 {
        return Err("snapshot file is too short".to_string());
    }
    if &input[..5] != RDB_MAGIC_PREFIX {
        return Err("snapshot is not a Redis/Valkey RDB file".to_string());
    }
    let version_raw = std::str::from_utf8(&input[5..9]).map_err(|_| "invalid RDB header")?;
    let _version = version_raw
        .parse::<u32>()
        .map_err(|_| format!("invalid RDB version '{version_raw}'"))?;

    let mut cursor = 9usize;
    let mut expire_at_s: Option<u32> = None;
    let mut entries = Vec::new();

    while cursor < input.len() {
        let op = read_u8(input, &mut cursor)?;
        match op {
            OP_EOF => break,
            OP_SELECTDB => {
                let _ = decode_len(input, &mut cursor)?;
                expire_at_s = None;
            }
            OP_EXPIRETIME => {
                let mut buf = [0u8; 4];
                read_exact(input, &mut cursor, &mut buf)?;
                expire_at_s = Some(u32::from_le_bytes(buf));
            }
            0xFC => {
                let mut buf = [0u8; 8];
                read_exact(input, &mut cursor, &mut buf)?;
                let expire_ms = u64::from_le_bytes(buf);
                let expire_s = expire_ms.div_ceil(1000);
                let expire_s_u32 = u32::try_from(expire_s)
                    .map_err(|_| "RDB millisecond expiration out of range".to_string())?;
                expire_at_s = Some(expire_s_u32);
            }
            0xFA => {
                let _ = decode_string(input, &mut cursor)?;
                let _ = decode_string(input, &mut cursor)?;
            }
            0xFB => {
                let _ = decode_len(input, &mut cursor)?;
                let _ = decode_len(input, &mut cursor)?;
            }
            TYPE_STRING | TYPE_LIST | TYPE_SET | TYPE_ZSET | TYPE_HASH => {
                let key = decode_string(input, &mut cursor)?;
                let value = match op {
                    TYPE_STRING => {
                        let raw = decode_string(input, &mut cursor)?;
                        if raw.starts_with(EMBEDDED_PREFIX) {
                            let payload = &raw[EMBEDDED_PREFIX.len()..];
                            decode_custom_entry(payload)?
                        } else {
                            Value::String(raw)
                        }
                    }
                    TYPE_LIST => {
                        let len = decode_len(input, &mut cursor)?;
                        let mut values = Vec::with_capacity(len);
                        for _ in 0..len {
                            values.push(decode_string(input, &mut cursor)?);
                        }
                        Value::List(values)
                    }
                    TYPE_SET => {
                        let len = decode_len(input, &mut cursor)?;
                        let mut values = Vec::with_capacity(len);
                        for _ in 0..len {
                            values.push(decode_string(input, &mut cursor)?);
                        }
                        Value::Set(values)
                    }
                    TYPE_ZSET => {
                        let len = decode_len(input, &mut cursor)?;
                        let mut values = Vec::with_capacity(len);
                        for _ in 0..len {
                            let member = decode_string(input, &mut cursor)?;
                            let score = decode_zset_score(input, &mut cursor)?;
                            values.push((member, score));
                        }
                        Value::ZSet(values)
                    }
                    TYPE_HASH => {
                        let len = decode_len(input, &mut cursor)?;
                        let mut values = Vec::with_capacity(len);
                        for _ in 0..len {
                            let field = decode_string(input, &mut cursor)?;
                            let value = decode_string(input, &mut cursor)?;
                            values.push((field, value));
                        }
                        Value::Hash(values)
                    }
                    _ => unreachable!(),
                };

                entries.push(LoadedEntry {
                    key,
                    expire_at_s,
                    value,
                });
                expire_at_s = None;
            }
            _ => return Err(format!("unsupported RDB opcode/type: {op}")),
        }
    }

    Ok(entries)
}

fn encode_len<W: Write>(out: &mut W, len: usize) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::encode_len");
    if len < (1 << 6) {
        out.write_all(&[len as u8]).map_err(|err| err.to_string())?;
        return Ok(());
    }
    if len < (1 << 14) {
        out.write_all(&[((len >> 8) as u8 & 0x3F) | 0b0100_0000, (len & 0xFF) as u8])
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    let len_u32 = u32::try_from(len).map_err(|_| "RDB length over u32 is unsupported")?;
    out.write_all(&[0b1000_0000])
        .and_then(|_| out.write_all(&len_u32.to_be_bytes()))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn decode_len(input: &[u8], cursor: &mut usize) -> Result<usize, String> {
    let _trace = profiler::scope("server::backup::decode_len");
    let first = read_u8(input, cursor)?;
    decode_len_with_first(input, cursor, first)
}

fn encode_string<W: Write>(out: &mut W, value: &[u8]) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::encode_string");
    encode_len(out, value.len())?;
    out.write_all(value).map_err(|err| err.to_string())?;
    Ok(())
}

fn decode_string(input: &[u8], cursor: &mut usize) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::decode_string");
    let first = read_u8(input, cursor)?;
    let mode = first >> 6;
    if mode != 0b11 {
        let len = decode_len_with_first(input, cursor, first)?;
        let mut out = vec![0u8; len];
        read_exact(input, cursor, &mut out)?;
        return Ok(out);
    }

    match first & 0x3F {
        0 => {
            let value = read_u8(input, cursor)? as i8;
            Ok(value.to_string().into_bytes())
        }
        1 => {
            let mut buf = [0u8; 2];
            read_exact(input, cursor, &mut buf)?;
            let value = i16::from_le_bytes(buf);
            Ok(value.to_string().into_bytes())
        }
        2 => {
            let mut buf = [0u8; 4];
            read_exact(input, cursor, &mut buf)?;
            let value = i32::from_le_bytes(buf);
            Ok(value.to_string().into_bytes())
        }
        3 => {
            let compressed_len = decode_len(input, cursor)?;
            let uncompressed_len = decode_len(input, cursor)?;
            let mut compressed = vec![0u8; compressed_len];
            read_exact(input, cursor, &mut compressed)?;
            lzf_decompress(&compressed, uncompressed_len)
        }
        _ => Err("unsupported RDB string encoding".to_string()),
    }
}

fn decode_len_with_first(input: &[u8], cursor: &mut usize, first: u8) -> Result<usize, String> {
    let _trace = profiler::scope("server::backup::decode_len_with_first");
    let mode = first >> 6;
    match mode {
        0b00 => Ok((first & 0x3F) as usize),
        0b01 => {
            let second = read_u8(input, cursor)?;
            Ok((((first & 0x3F) as usize) << 8) | second as usize)
        }
        0b10 => match first & 0x3F {
            0 => {
                let mut buf = [0u8; 4];
                read_exact(input, cursor, &mut buf)?;
                Ok(u32::from_be_bytes(buf) as usize)
            }
            1 => {
                let mut buf = [0u8; 8];
                read_exact(input, cursor, &mut buf)?;
                usize::try_from(u64::from_be_bytes(buf))
                    .map_err(|_| "RDB 64-bit length does not fit usize".to_string())
            }
            _ => Err("unsupported RDB length encoding".to_string()),
        },
        _ => Err("RDB encoded value is not a plain length".to_string()),
    }
}

fn lzf_decompress(input: &[u8], expected_len: usize) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::lzf_decompress");
    let mut out = Vec::with_capacity(expected_len);
    let mut i = 0usize;

    while i < input.len() {
        let ctrl = input[i] as usize;
        i += 1;

        if ctrl < 32 {
            let len = ctrl + 1;
            if i + len > input.len() {
                return Err("invalid LZF literal length".to_string());
            }
            out.extend_from_slice(&input[i..i + len]);
            i += len;
            continue;
        }

        let mut len = (ctrl >> 5) + 2;
        let mut ref_offset = (ctrl & 0x1F) << 8;
        if i >= input.len() {
            return Err("invalid LZF back-reference".to_string());
        }
        ref_offset += input[i] as usize;
        i += 1;

        if len == 9 {
            if i >= input.len() {
                return Err("invalid LZF extended length".to_string());
            }
            len += input[i] as usize;
            i += 1;
        }

        let back = ref_offset + 1;
        if back > out.len() {
            return Err("invalid LZF match distance".to_string());
        }
        let start = out.len() - back;
        for j in 0..len {
            let b = out[start + j];
            out.push(b);
        }
    }

    if out.len() != expected_len {
        return Err(format!(
            "invalid LZF output length: expected {expected_len}, got {}",
            out.len()
        ));
    }

    Ok(out)
}

fn encode_zset_score<W: Write>(out: &mut W, score: f64) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::encode_zset_score");
    if score.is_nan() {
        out.write_all(&[253]).map_err(|err| err.to_string())?;
        return Ok(());
    }
    if score == f64::INFINITY {
        out.write_all(&[254]).map_err(|err| err.to_string())?;
        return Ok(());
    }
    if score == f64::NEG_INFINITY {
        out.write_all(&[255]).map_err(|err| err.to_string())?;
        return Ok(());
    }

    let encoded = score.to_string();
    out.write_all(&[encoded.len() as u8])
        .and_then(|_| out.write_all(encoded.as_bytes()))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn decode_zset_score(input: &[u8], cursor: &mut usize) -> Result<f64, String> {
    let _trace = profiler::scope("server::backup::decode_zset_score");
    let len = read_u8(input, cursor)?;
    match len {
        253 => Ok(f64::NAN),
        254 => Ok(f64::INFINITY),
        255 => Ok(f64::NEG_INFINITY),
        n => {
            let mut bytes = vec![0u8; n as usize];
            read_exact(input, cursor, &mut bytes)?;
            let raw = std::str::from_utf8(&bytes).map_err(|_| "invalid zset score utf8")?;
            raw.parse::<f64>()
                .map_err(|_| format!("invalid zset score '{raw}'"))
        }
    }
}

fn read_u8(input: &[u8], cursor: &mut usize) -> Result<u8, String> {
    let _trace = profiler::scope("server::backup::read_u8");
    if *cursor >= input.len() {
        return Err("unexpected EOF".to_string());
    }
    let b = input[*cursor];
    *cursor += 1;
    Ok(b)
}

fn read_exact(input: &[u8], cursor: &mut usize, out: &mut [u8]) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::read_exact");
    let remaining = input.len().saturating_sub(*cursor);
    if out.len() > remaining {
        return Err("unexpected EOF".to_string());
    }
    out.copy_from_slice(&input[*cursor..*cursor + out.len()]);
    *cursor += out.len();
    Ok(())
}

fn now_unix_seconds() -> u64 {
    let _trace = profiler::scope("server::backup::now_unix_seconds");
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decode_custom_entry(payload: &[u8]) -> Result<Value, String> {
    let _trace = profiler::scope("server::backup::decode_custom_entry");
    if payload.len() < 2 || payload[0] != 1 {
        return Err("invalid payload".to_string());
    }

    let mut input = &payload[2..];
    let value = match payload[1] {
        0 => Value::String(read_bytes(&mut input)?),
        1 => {
            let count = read_u32(&mut input)? as usize;
            let mut map: HashMap<Vec<u8>, Vec<u8>, RandomState> =
                HashMap::with_capacity_and_hasher(count, RandomState::new());
            for _ in 0..count {
                let field = read_bytes(&mut input)?;
                let value = read_bytes(&mut input)?;
                map.insert(field, value);
            }
            Value::Hash(map.into_iter().collect())
        }
        2 => {
            let count = read_u32(&mut input)? as usize;
            let mut list = VecDeque::with_capacity(count);
            for _ in 0..count {
                list.push_back(read_bytes(&mut input)?);
            }
            Value::List(list.into_iter().collect())
        }
        3 => {
            let count = read_u32(&mut input)? as usize;
            let mut set: IndexSet<Vec<u8>, RandomState> =
                IndexSet::with_capacity_and_hasher(count, RandomState::new());
            for _ in 0..count {
                set.insert(read_bytes(&mut input)?);
            }
            Value::Set(set.into_iter().collect())
        }
        4 => {
            let count = read_u32(&mut input)? as usize;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                let member = read_bytes(&mut input)?;
                if input.len() < 8 {
                    return Err("invalid zset payload".to_string());
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&input[..8]);
                input = &input[8..];
                values.push((member, f64::from_le_bytes(bytes)));
            }
            Value::ZSet(values)
        }
        5 => {
            let count = read_u32(&mut input)? as usize;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                let member = read_bytes(&mut input)?;
                if input.len() < 16 {
                    return Err("invalid geo payload".to_string());
                }
                let mut lon = [0u8; 8];
                lon.copy_from_slice(&input[..8]);
                let mut lat = [0u8; 8];
                lat.copy_from_slice(&input[8..16]);
                input = &input[16..];
                values.push((member, (f64::from_le_bytes(lon), f64::from_le_bytes(lat))));
            }
            Value::Geo(values)
        }
        6 => {
            let count = read_u32(&mut input)? as usize;
            let mut entries = BTreeMap::new();
            for _ in 0..count {
                if input.len() < 16 {
                    return Err("invalid stream payload".to_string());
                }
                let mut ms = [0u8; 8];
                ms.copy_from_slice(&input[..8]);
                let mut seq = [0u8; 8];
                seq.copy_from_slice(&input[8..16]);
                input = &input[16..];
                let field_count = read_u32(&mut input)? as usize;
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    fields.push((read_bytes(&mut input)?, read_bytes(&mut input)?));
                }
                entries.insert(
                    (u64::from_le_bytes(ms), u64::from_le_bytes(seq)),
                    StreamEntry {
                        ms: u64::from_le_bytes(ms),
                        seq: u64::from_le_bytes(seq),
                        fields,
                    },
                );
            }
            Value::Stream(entries.into_values().collect())
        }
        _ => return Err("unsupported payload type".to_string()),
    };

    if !input.is_empty() {
        return Err("payload has trailing bytes".to_string());
    }
    Ok(value)
}

fn read_u32(input: &mut &[u8]) -> Result<u32, String> {
    let _trace = profiler::scope("server::backup::read_u32");
    if input.len() < 4 {
        return Err("unexpected EOF".to_string());
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&input[..4]);
    *input = &input[4..];
    Ok(u32::from_le_bytes(bytes))
}

fn read_bytes(input: &mut &[u8]) -> Result<Vec<u8>, String> {
    let _trace = profiler::scope("server::backup::read_bytes");
    let len = read_u32(input)? as usize;
    if input.len() < len {
        return Err("unexpected EOF".to_string());
    }
    let out = input[..len].to_vec();
    *input = &input[len..];
    Ok(out)
}

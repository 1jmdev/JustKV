use std::collections::{BTreeMap, VecDeque};
use std::io::Write;

use ahash::RandomState;
use hashbrown::HashMap;
use indexmap::IndexSet;

pub(super) const RDB_MAGIC_PREFIX: &[u8; 5] = b"REDIS";
pub(super) const RDB_VERSION: &str = "0004";
pub(super) const OP_EXPIRETIME: u8 = 0xFD;
pub(super) const OP_SELECTDB: u8 = 0xFE;
pub(super) const OP_EOF: u8 = 0xFF;
pub(super) const TYPE_STRING: u8 = 0;
pub(super) const TYPE_LIST: u8 = 1;
pub(super) const TYPE_SET: u8 = 2;
pub(super) const TYPE_ZSET: u8 = 3;
pub(super) const TYPE_HASH: u8 = 4;
pub(super) const EMBEDDED_PREFIX: &[u8] = b"__BETTERKV_EMBEDDED__";

/// In-memory representation of a value loaded from or written to a snapshot.
#[derive(Debug, Clone)]
pub(super) enum Value {
    String(Vec<u8>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    ZSet(Vec<(Vec<u8>, f64)>),
    Geo(Vec<(Vec<u8>, (f64, f64))>),
    Stream(Vec<StreamEntry>),
}

#[derive(Debug, Clone)]
pub(super) struct StreamEntry {
    pub(super) ms: u64,
    pub(super) seq: u64,
    pub(super) fields: Vec<(Vec<u8>, Vec<u8>)>,
}

#[derive(Debug)]
pub(super) struct LoadedEntry {
    pub(super) key: Vec<u8>,
    pub(super) expire_at_s: Option<u32>,
    pub(super) value: Value,
}

/// Wire representation used only when writing an RDB file.
pub(super) enum RdbValue {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    ZSet(Vec<(Vec<u8>, f64)>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
}

pub(super) fn write_rdb_value<W: Write>(
    out: &mut W,
    key: &[u8],
    value: RdbValue,
) -> Result<(), String> {
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

pub(super) fn encode_len<W: Write>(out: &mut W, len: usize) -> Result<(), String> {
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

pub(super) fn encode_string<W: Write>(out: &mut W, value: &[u8]) -> Result<(), String> {
    let _trace = profiler::scope("server::backup::encode_string");
    encode_len(out, value.len())?;
    out.write_all(value).map_err(|err| err.to_string())?;
    Ok(())
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

// ---------------------------------------------------------------------------
// Counting writer
// ---------------------------------------------------------------------------

pub(super) struct CountingWriter<W> {
    inner: W,
    bytes_written: u64,
}

impl<W> CountingWriter<W> {
    pub(super) fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }

    pub(super) fn into_inner(self) -> W {
        self.inner
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

pub(super) fn parse_rdb(input: &[u8]) -> Result<Vec<LoadedEntry>, String> {
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
                decode_len(input, &mut cursor)?;
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
                decode_string(input, &mut cursor)?;
                decode_string(input, &mut cursor)?;
            }
            0xFB => {
                decode_len(input, &mut cursor)?;
                decode_len(input, &mut cursor)?;
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

fn decode_len(input: &[u8], cursor: &mut usize) -> Result<usize, String> {
    let _trace = profiler::scope("server::backup::decode_len");
    let first = read_u8(input, cursor)?;
    decode_len_with_first(input, cursor, first)
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

pub(super) fn decode_custom_entry(payload: &[u8]) -> Result<Value, String> {
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

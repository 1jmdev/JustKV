use crate::store::{BitFieldEncoding, BitFieldOp, BitFieldOverflow, BitOp, Store};
use crate::value::{CompactArg, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

impl Store {
    pub fn getbit(&self, key: &[u8], offset: usize) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::getbit");
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let Some(entry) = shard.entries.get::<[u8]>(key) else {
            return Ok(0);
        };
        if shard
            .ttl
            .get(key)
            .copied()
            .is_some_and(|deadline| now_ms >= deadline)
        {
            return Ok(0);
        }
        let value = entry.as_string().ok_or(())?;
        Ok(i64::from(read_single_bit(value.as_slice(), offset)))
    }

    pub fn setbit(&self, key: &[u8], offset: usize, bit: u8) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::setbit");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let mut data = if purge_if_expired(&mut shard, key, now_ms) {
            Vec::new()
        } else {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => entry.as_string().ok_or(())?.to_vec(),
                None => Vec::new(),
            }
        };
        let ttl_deadline = shard.ttl.get(key).copied();

        let previous = read_single_bit(&data, offset);
        write_single_bit(&mut data, offset, bit);
        write_entry(&mut shard, key, Entry::new(data), ttl_deadline);
        Ok(i64::from(previous))
    }

    pub fn bitcount(
        &self,
        key: &[u8],
        start: Option<i64>,
        end: Option<i64>,
        bit_unit: bool,
    ) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::bitcount");
        let Some(value) = self.get(key)? else {
            return Ok(0);
        };
        let data = value.as_slice();
        if data.is_empty() {
            return Ok(0);
        }

        if bit_unit {
            let total_bits = data.len().saturating_mul(8);
            let Some((from, to_exclusive)) = normalize_slice(
                start.unwrap_or(0),
                end.unwrap_or((total_bits as i64) - 1),
                total_bits,
            ) else {
                return Ok(0);
            };
            return Ok(count_bits_in_bit_range(data, from, to_exclusive) as i64);
        }

        let Some((from, to_exclusive)) = normalize_slice(
            start.unwrap_or(0),
            end.unwrap_or((data.len() as i64) - 1),
            data.len(),
        ) else {
            return Ok(0);
        };

        Ok(data[from..to_exclusive]
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum::<usize>() as i64)
    }

    pub fn bitpos(
        &self,
        key: &[u8],
        bit: u8,
        start: Option<i64>,
        end: Option<i64>,
        bit_unit: bool,
    ) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::bitpos");
        let Some(value) = self.get(key)? else {
            return Ok(if bit == 0 && start.is_none() { 0 } else { -1 });
        };
        let data = value.as_slice();
        if data.is_empty() {
            return Ok(if bit == 0 && start.is_none() { 0 } else { -1 });
        }

        let default_end = if bit_unit {
            (data.len().saturating_mul(8) as i64) - 1
        } else {
            (data.len() as i64) - 1
        };
        let Some((from, to_exclusive)) = normalize_slice(
            start.unwrap_or(0),
            end.unwrap_or(default_end),
            if bit_unit {
                data.len().saturating_mul(8)
            } else {
                data.len()
            },
        ) else {
            return Ok(-1);
        };

        let scan = if bit_unit {
            find_bit_in_bit_range(data, bit, from, to_exclusive).map(|idx| idx as i64)
        } else {
            let bit_from = from.saturating_mul(8);
            let bit_to = to_exclusive.saturating_mul(8);
            find_bit_in_bit_range(data, bit, bit_from, bit_to).map(|idx| idx as i64)
        };

        if let Some(position) = scan {
            return Ok(position);
        }

        if bit == 0 && start.is_none() && end.is_none() && !bit_unit {
            return Ok((data.len().saturating_mul(8)) as i64);
        }

        Ok(-1)
    }

    pub fn bitop(&self, op: BitOp, destination: &[u8], keys: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::bitop");
        let mut values = Vec::with_capacity(keys.len());
        let mut max_len = 0usize;

        for key in keys {
            let data = self.get(key.as_slice())?.map(|value| value.to_vec());
            if let Some(ref bytes) = data {
                max_len = max_len.max(bytes.len());
            }
            values.push(data);
        }

        let result = match op {
            BitOp::Not => {
                let source = values
                    .first()
                    .and_then(|value| value.as_ref())
                    .cloned()
                    .unwrap_or_default();
                source.into_iter().map(|byte| !byte).collect::<Vec<u8>>()
            }
            BitOp::And => {
                let mut out = vec![0xFFu8; max_len];
                for value in &values {
                    for (idx, out_byte) in out.iter_mut().enumerate() {
                        *out_byte &= value
                            .as_ref()
                            .and_then(|bytes| bytes.get(idx))
                            .copied()
                            .unwrap_or(0);
                    }
                }
                out
            }
            BitOp::Or => {
                let mut out = vec![0u8; max_len];
                for value in &values {
                    for (idx, out_byte) in out.iter_mut().enumerate() {
                        *out_byte |= value
                            .as_ref()
                            .and_then(|bytes| bytes.get(idx))
                            .copied()
                            .unwrap_or(0);
                    }
                }
                out
            }
            BitOp::Xor => {
                let mut out = vec![0u8; max_len];
                for value in &values {
                    for (idx, out_byte) in out.iter_mut().enumerate() {
                        *out_byte ^= value
                            .as_ref()
                            .and_then(|bytes| bytes.get(idx))
                            .copied()
                            .unwrap_or(0);
                    }
                }
                out
            }
        };

        self.set(destination, &result, None);
        Ok(result.len() as i64)
    }

    pub fn bitfield(&self, key: &[u8], ops: &[BitFieldOp]) -> Result<Vec<Option<i64>>, ()> {
        let _trace = profiler::scope("crates::engine::src::strings::bitmap::bitfield");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let mut data = if purge_if_expired(&mut shard, key, now_ms) {
            Vec::new()
        } else {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => entry.as_string().ok_or(())?.to_vec(),
                None => Vec::new(),
            }
        };
        let ttl_deadline = shard.ttl.get(key).copied();
        let mut mutated = false;
        let mut out = Vec::with_capacity(ops.len());

        for op in ops {
            match *op {
                BitFieldOp::Get { encoding, offset } => {
                    out.push(Some(read_bitfield_value(&data, encoding, offset)));
                }
                BitFieldOp::Set {
                    encoding,
                    offset,
                    value,
                } => {
                    let previous = read_bitfield_value(&data, encoding, offset);
                    let encoded = encode_for_storage(encoding, value);
                    write_bitfield_value(&mut data, encoding, offset, encoded);
                    mutated = true;
                    out.push(Some(previous));
                }
                BitFieldOp::IncrBy {
                    encoding,
                    offset,
                    increment,
                    overflow,
                } => {
                    let current = read_bitfield_value(&data, encoding, offset) as i128;
                    let (min, max) = encoding_limits(encoding);
                    let candidate = current + i128::from(increment);
                    let next = if candidate < min || candidate > max {
                        match overflow {
                            BitFieldOverflow::Wrap => wrap_to_range(candidate, min, max),
                            BitFieldOverflow::Sat => candidate.clamp(min, max),
                            BitFieldOverflow::Fail => {
                                out.push(None);
                                continue;
                            }
                        }
                    } else {
                        candidate
                    };

                    write_bitfield_value(
                        &mut data,
                        encoding,
                        offset,
                        encode_for_storage(encoding, next as i64),
                    );
                    mutated = true;
                    out.push(Some(next as i64));
                }
            }
        }

        if mutated {
            write_entry(&mut shard, key, Entry::new(data), ttl_deadline);
        }

        Ok(out)
    }
}

fn read_single_bit(data: &[u8], offset: usize) -> u8 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::read_single_bit");
    let byte_index = offset / 8;
    let bit_index = 7 - (offset % 8);
    data.get(byte_index)
        .map(|byte| (byte >> bit_index) & 1)
        .unwrap_or(0)
}

fn write_single_bit(data: &mut Vec<u8>, offset: usize, bit: u8) {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::write_single_bit");
    let byte_index = offset / 8;
    let bit_index = 7 - (offset % 8);
    if data.len() <= byte_index {
        data.resize(byte_index + 1, 0);
    }
    let mask = 1u8 << bit_index;
    if bit == 0 {
        data[byte_index] &= !mask;
    } else {
        data[byte_index] |= mask;
    }
}

fn normalize_slice(start: i64, end: i64, len: usize) -> Option<(usize, usize)> {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::normalize_slice");
    if len == 0 {
        return None;
    }

    let len_i64 = len as i64;
    let mut from = if start < 0 { len_i64 + start } else { start };
    let mut to = if end < 0 { len_i64 + end } else { end };

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

fn count_bits_in_bit_range(data: &[u8], from_bit: usize, to_bit_exclusive: usize) -> usize {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::count_bits_in_bit_range");
    if from_bit >= to_bit_exclusive {
        return 0;
    }
    let mut count = 0usize;
    for bit in from_bit..to_bit_exclusive {
        count += usize::from(read_single_bit(data, bit));
    }
    count
}

fn find_bit_in_bit_range(
    data: &[u8],
    target: u8,
    from_bit: usize,
    to_bit_exclusive: usize,
) -> Option<usize> {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::find_bit_in_bit_range");
    (from_bit..to_bit_exclusive).find(|offset| read_single_bit(data, *offset) == target)
}

fn encoding_bits(encoding: BitFieldEncoding) -> u8 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::encoding_bits");
    match encoding {
        BitFieldEncoding::Signed { bits } | BitFieldEncoding::Unsigned { bits } => bits,
    }
}

fn encoding_limits(encoding: BitFieldEncoding) -> (i128, i128) {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::encoding_limits");
    match encoding {
        BitFieldEncoding::Signed { bits } => {
            let max = (1i128 << (bits - 1)) - 1;
            (-(1i128 << (bits - 1)), max)
        }
        BitFieldEncoding::Unsigned { bits } => (0, (1i128 << bits) - 1),
    }
}

fn wrap_to_range(value: i128, min: i128, max: i128) -> i128 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::wrap_to_range");
    let span = max - min + 1;
    min + (value - min).rem_euclid(span)
}

fn read_bitfield_value(data: &[u8], encoding: BitFieldEncoding, offset: usize) -> i64 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::read_bitfield_value");
    let bits = encoding_bits(encoding) as usize;
    let raw = read_uint(data, offset, bits);

    match encoding {
        BitFieldEncoding::Unsigned { .. } => raw as i64,
        BitFieldEncoding::Signed { bits } => {
            if bits == 64 {
                raw as i64
            } else {
                let sign = 1u64 << (bits - 1);
                if (raw & sign) == 0 {
                    raw as i64
                } else {
                    (i128::from(raw) - (1i128 << bits)) as i64
                }
            }
        }
    }
}

fn encode_for_storage(encoding: BitFieldEncoding, value: i64) -> u64 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::encode_for_storage");
    let bits = encoding_bits(encoding);
    if bits == 64 {
        return value as u64;
    }

    let modulus = 1i128 << bits;
    let wrapped = i128::from(value).rem_euclid(modulus);
    wrapped as u64
}

fn write_bitfield_value(data: &mut Vec<u8>, encoding: BitFieldEncoding, offset: usize, raw: u64) {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::write_bitfield_value");
    write_uint(data, offset, encoding_bits(encoding) as usize, raw);
}

fn read_uint(data: &[u8], offset: usize, bits: usize) -> u64 {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::read_uint");
    let mut value = 0u64;
    for step in 0..bits {
        value = (value << 1) | u64::from(read_single_bit(data, offset + step));
    }
    value
}

fn write_uint(data: &mut Vec<u8>, offset: usize, bits: usize, value: u64) {
    let _trace = profiler::scope("crates::engine::src::strings::bitmap::write_uint");
    for step in 0..bits {
        let shift = bits - 1 - step;
        let bit = ((value >> shift) & 1) as u8;
        write_single_bit(data, offset + step, bit);
    }
}

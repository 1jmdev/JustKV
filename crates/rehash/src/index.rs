const WY_SECRET_0: u64 = 0xa076_1d64_78bd_642f;
const WY_SECRET_1: u64 = 0xe703_7ed1_a0b4_28db;
const WY_SECRET_2: u64 = 0x8ebc_6af0_9c88_c6e3;

#[inline(always)]
fn wymix(a: u64, b: u64) -> u64 {
    let _trace = profiler::scope("rehash::index::wymix");
    let r = (a as u128).wrapping_mul(b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

#[inline(always)]
fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
    let _trace = profiler::scope("rehash::index::read_u32_le");
    // Safety: Caller logic guarantees `off + 4 <= bytes.len()`
    unsafe {
        u32::from_le_bytes(std::ptr::read_unaligned(
            bytes.as_ptr().add(off) as *const [u8; 4]
        ))
    }
}

#[inline(always)]
fn read_u64_le(bytes: &[u8], off: usize) -> u64 {
    let _trace = profiler::scope("rehash::index::read_u64_le");
    // Safety: Caller logic guarantees `off + 8 <= bytes.len()`
    unsafe {
        u64::from_le_bytes(std::ptr::read_unaligned(
            bytes.as_ptr().add(off) as *const [u8; 8]
        ))
    }
}

#[inline(always)]
pub(super) fn hash_key(seed: u64, key: &[u8]) -> u32 {
    let _trace = profiler::scope("rehash::index::hash_key");
    let len = key.len();

    if len <= 16 {
        let a;
        let b;
        if (4..8).contains(&len) {
            let first = read_u32_le(key, 0) as u64;
            let last = read_u32_le(key, len - 4) as u64;
            a = (first << 32) | first;
            b = (last << 32) | last;
        } else if len >= 4 {
            let half = (len >> 3) << 2;
            a = (read_u32_le(key, 0) as u64) << 32 | read_u32_le(key, half) as u64;
            b = (read_u32_le(key, len - 4) as u64) << 32 | read_u32_le(key, len - 4 - half) as u64;
        } else if len > 0 {
            a = key[0] as u64 | ((key[len >> 1] as u64) << 8) | ((key[len - 1] as u64) << 16);
            b = len as u64;
        } else {
            return wymix(seed ^ WY_SECRET_0, WY_SECRET_1) as u32;
        }
        return wymix(WY_SECRET_1 ^ (len as u64), wymix(a ^ WY_SECRET_1, b ^ seed)) as u32;
    }

    let mut off = 0usize;
    let mut rem = len;
    let mut s = seed;

    while rem > 16 {
        s = wymix(
            read_u64_le(key, off) ^ WY_SECRET_1,
            read_u64_le(key, off + 8) ^ s,
        );
        off += 16;
        rem -= 16;
    }

    let a = read_u64_le(key, len - 16);
    let b = read_u64_le(key, len - 8);
    wymix(WY_SECRET_2 ^ (len as u64), wymix(a ^ WY_SECRET_1, b ^ s)) as u32
}

use crate::store::Store;

use super::super::helpers::{get_live_entry, monotonic_now_ms};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LcsMatch {
    pub first: (usize, usize),
    pub second: (usize, usize),
    pub len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LcsResult {
    pub sequence: Vec<u8>,
    pub matches: Vec<LcsMatch>,
}

impl Store {
    pub fn lcs(&self, first_key: &[u8], second_key: &[u8]) -> Result<LcsResult, ()> {
        let _trace = profiler::scope("engine::strings::lcs::lcs");
        let first = self.get_string_or_empty(first_key)?;
        let second = self.get_string_or_empty(second_key)?;
        Ok(compute_lcs(&first, &second))
    }

    fn get_string_or_empty(&self, key: &[u8]) -> Result<Vec<u8>, ()> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let Some(entry) = get_live_entry(&shard, key, now_ms) else {
            return Ok(Vec::new());
        };
        let Some(value) = entry.as_string() else {
            return Err(());
        };
        Ok(value.to_vec())
    }
}

fn compute_lcs(first: &[u8], second: &[u8]) -> LcsResult {
    let rows = first.len();
    let cols = second.len();
    let mut table = vec![0usize; (rows + 1) * (cols + 1)];

    for i in (0..rows).rev() {
        for j in (0..cols).rev() {
            let idx = i * (cols + 1) + j;
            table[idx] = if first[i] == second[j] {
                table[(i + 1) * (cols + 1) + (j + 1)] + 1
            } else {
                table[(i + 1) * (cols + 1) + j].max(table[i * (cols + 1) + (j + 1)])
            };
        }
    }

    let mut i = 0usize;
    let mut j = 0usize;
    let mut sequence = Vec::with_capacity(table[0]);
    let mut matches = Vec::new();

    while i < rows && j < cols {
        if first[i] == second[j] {
            let start_i = i;
            let start_j = j;
            while i < rows && j < cols && first[i] == second[j] {
                sequence.push(first[i]);
                i += 1;
                j += 1;
                if i >= rows || j >= cols {
                    break;
                }
                let current = table[i * (cols + 1) + j];
                let diagonal = table[(i + 1) * (cols + 1) + (j + 1)];
                if first[i] != second[j] || current != diagonal + 1 {
                    break;
                }
            }
            let length = i - start_i;
            if length != 0 {
                matches.push(LcsMatch {
                    first: (start_i, i - 1),
                    second: (start_j, j - 1),
                    len: length,
                });
            }
            continue;
        }

        let down = table[(i + 1) * (cols + 1) + j];
        let right = table[i * (cols + 1) + (j + 1)];
        if down >= right {
            i += 1;
        } else {
            j += 1;
        }
    }

    matches.reverse();
    LcsResult { sequence, matches }
}

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rehash::RehashingMap;
use std::hint::black_box;

fn make_keys(n: usize, key_len: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| {
            let s = format!("{:0>width$}", i, width = key_len);
            s.into_bytes()
        })
        .collect()
}

fn prefilled_map(n: usize, key_len: usize) -> RehashingMap<Vec<u8>, u64> {
    let mut map = RehashingMap::new();
    for (i, key) in make_keys(n, key_len).into_iter().enumerate() {
        map.insert(key, i as u64);
    }
    map
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    for &n in &[100usize, 10_000, 1_000_000] {
        for &key_len in &[8usize, 32, 128] {
            let keys = make_keys(n, key_len);
            group.bench_with_input(
                BenchmarkId::new(format!("key{key_len}"), n),
                &(n, key_len),
                |b, _| {
                    b.iter(|| {
                        let mut map: RehashingMap<Vec<u8>, u64> = RehashingMap::new();
                        for (i, key) in keys.iter().enumerate() {
                            map.insert(black_box(key.clone()), black_box(i as u64));
                        }
                        map
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("get");
    for &n in &[100usize, 10_000, 1_000_000] {
        for &key_len in &[8usize, 32, 128] {
            let map = prefilled_map(n, key_len);
            let keys = make_keys(n, key_len);
            group.bench_with_input(
                BenchmarkId::new(format!("key{key_len}"), n),
                &(n, key_len),
                |b, _| {
                    b.iter(|| {
                        let mut sum = 0u64;
                        for key in &keys {
                            if let Some(&v) = map.get(black_box(key.as_slice())) {
                                sum = sum.wrapping_add(v);
                            }
                        }
                        sum
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove");
    for &n in &[100usize, 10_000] {
        for &key_len in &[8usize, 32, 128] {
            let keys = make_keys(n, key_len);
            group.bench_with_input(
                BenchmarkId::new(format!("key{key_len}"), n),
                &(n, key_len),
                |b, _| {
                    b.iter(|| {
                        let mut map = prefilled_map(n, key_len);
                        for key in &keys {
                            map.remove(black_box(key.as_slice()));
                        }
                    });
                },
            );
        }
    }
    group.finish();
}

fn bench_get_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_batch");
    for &n in &[10_000usize, 1_000_000] {
        for &key_len in &[8usize, 32, 128] {
            let map = prefilled_map(n, key_len);
            let all_keys = make_keys(n, key_len);
            let probe_keys: Vec<Vec<u8>> = (0..8).map(|i| all_keys[i * (n / 8)].clone()).collect();
            let probe_refs: [&[u8]; 8] = std::array::from_fn(|i| probe_keys[i].as_slice());

            group.bench_with_input(
                BenchmarkId::new(format!("key{key_len}"), n),
                &(n, key_len),
                |b, _| {
                    b.iter(|| black_box(map.get_batch(black_box(&probe_refs))));
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_get,
    bench_remove,
    bench_get_batch
);
criterion_main!(benches);

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use betterkv_benchmark::benchmark::{
    RandomSource, build_cumulative_distribution, build_mset_command, build_request_group,
    build_setup_command, make_key,
};
use betterkv_benchmark::workload::{BenchKind, BenchRun};

fn sample_run(kind: BenchKind, pipeline: usize) -> BenchRun {
    BenchRun {
        name: format!("{kind:?}"),
        kind,
        clients: 50,
        requests: 100_000,
        data_size: 16,
        pipeline,
        random_keyspace_len: Some(10_000),
        dbnum: 0,
        keep_alive: true,
        key_prefix: "betterkv-benchmark".to_string(),
        seed: 42,
        command: None,
    }
}

fn bench_request_building(c: &mut Criterion) {
    let key_base = b"betterkv-benchmark:GET:0";
    let value = vec![b'x'; 16];
    let mut group = c.benchmark_group("request_building");

    for &(kind, batch) in &[
        (BenchKind::Set, 1usize),
        (BenchKind::Set, 64usize),
        (BenchKind::Get, 64usize),
        (BenchKind::Mset, 64usize),
    ] {
        let run = sample_run(kind, batch);
        group.bench_with_input(
            BenchmarkId::new(format!("{kind:?}"), batch),
            &batch,
            |b, _| {
                b.iter(|| {
                    let mut random = RandomSource::new(42);
                    black_box(
                        build_request_group(
                            black_box(&run),
                            black_box(key_base),
                            black_box(value.as_slice()),
                            black_box(batch),
                            &mut random,
                        )
                        .expect("build request group"),
                    );
                });
            },
        );
    }

    group.finish();
}

fn bench_setup_and_keys(c: &mut Criterion) {
    let key_base = b"betterkv-benchmark:LRANGE_600:0";
    let value = vec![b'x'; 16];
    let mut group = c.benchmark_group("setup_and_keys");

    group.bench_function("make_key_slot_0", |b| {
        b.iter(|| black_box(make_key(black_box(key_base), black_box(0))));
    });

    group.bench_function("make_key_slot_9999", |b| {
        b.iter(|| black_box(make_key(black_box(key_base), black_box(9_999))));
    });

    group.bench_function("build_mset_command", |b| {
        b.iter(|| {
            black_box(build_mset_command(
                black_box(key_base),
                black_box(77),
                black_box(value.as_slice()),
            ))
        });
    });

    group.bench_function("build_setup_command_lrange600", |b| {
        b.iter(|| {
            black_box(
                build_setup_command(
                    black_box(BenchKind::Lrange600),
                    black_box(key_base),
                    black_box(7),
                    black_box(value.as_slice()),
                )
                .expect("setup command"),
            );
        });
    });

    group.finish();
}

fn bench_post_processing(c: &mut Criterion) {
    let samples = (0..100_000u64)
        .map(|index| 50_000 + index * 7)
        .collect::<Vec<_>>();

    c.bench_function("build_cumulative_distribution_100k", |b| {
        b.iter(|| black_box(build_cumulative_distribution(black_box(samples.as_slice()))));
    });
}

criterion_group!(
    benches,
    bench_request_building,
    bench_setup_and_keys,
    bench_post_processing
);
criterion_main!(benches);

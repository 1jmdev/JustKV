use commands::dispatch::{dispatch_args, parse_command_into};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};
use std::{hint::black_box, time::Duration};
use types::value::CompactArg;

fn arg(s: &str) -> CompactArg {
    CompactArg::from_slice(s.as_bytes())
}

macro_rules! dispatch {
    ($store:expr, $($s:literal),+) => {{
        let args: &[CompactArg] = &[$(arg($s)),+];
        dispatch_args($store, args)
    }};
}

fn populated_store(n: usize) -> Store {
    let store = Store::new(16);
    for i in 0..n {
        let key = format!("key:{i}");
        let val = format!("val:{i}");
        let args: &[CompactArg] = &[arg("SET"), arg(&key), arg(&val)];
        dispatch_args(&store, args);
    }
    store
}

fn bench_dispatcher(c: &mut Criterion) {
    let mut g = c.benchmark_group("dispatcher");
    let store = populated_store(1);
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    g.bench_function("dispatch_get_hit", |b| {
        b.iter(|| dispatch!(black_box(&store), "GET", "key:0"))
    });

    g.bench_function("dispatch_get_miss", |b| {
        b.iter(|| dispatch!(black_box(&store), "GET", "no_such_key"))
    });

    g.bench_function("dispatch_set", |b| {
        b.iter(|| dispatch!(black_box(&store), "SET", "bench_key", "bench_val"))
    });

    g.bench_function("dispatch_ping", |b| {
        b.iter(|| dispatch!(black_box(&store), "PING"))
    });

    g.bench_function("dispatch_long_pexpireat", |b| {
        b.iter(|| {
            let args: &[CompactArg] = &[arg("PEXPIREAT"), arg("key:0"), arg("9999999999000")];
            dispatch_args(black_box(&store), args)
        })
    });

    g.bench_function("parse_command_into_set", |b| {
        b.iter(|| {
            let frame = RespFrame::Array(Some(vec![
                RespFrame::Bulk(Some(BulkData::Arg(arg("SET")))),
                RespFrame::Bulk(Some(BulkData::Arg(arg("mykey")))),
                RespFrame::Bulk(Some(BulkData::Arg(arg("myvalue")))),
            ]));
            let mut args = Vec::new();
            let _ = parse_command_into(black_box(frame), &mut args);
            black_box(args)
        })
    });

    g.finish();
}

fn bench_string_get_set(c: &mut Criterion) {
    let mut g = c.benchmark_group("string_get_set");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    {
        let store = Store::new(16);
        g.bench_function("set_simple", |b| {
            b.iter(|| dispatch!(black_box(&store), "SET", "k", "v"))
        });
    }

    {
        let store = Store::new(16);
        g.bench_function("set_with_ex", |b| {
            b.iter(|| {
                let args: &[CompactArg] = &[arg("SET"), arg("k"), arg("v"), arg("EX"), arg("3600")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = populated_store(1);
        g.bench_function("get_hit", |b| {
            b.iter(|| dispatch!(black_box(&store), "GET", "key:0"))
        });
    }

    {
        let store = Store::new(16);
        g.bench_function("get_miss", |b| {
            b.iter(|| dispatch!(black_box(&store), "GET", "missing"))
        });
    }

    {
        let store = Store::new(16);
        dispatch!(&store, "SET", "counter", "0");
        g.bench_function("incr", |b| {
            b.iter(|| dispatch!(black_box(&store), "INCR", "counter"))
        });
    }

    {
        let store = Store::new(16);
        dispatch!(&store, "SET", "counter", "0");
        g.bench_function("incrby", |b| {
            b.iter(|| {
                let args: &[CompactArg] = &[arg("INCRBY"), arg("counter"), arg("100")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = populated_store(1);
        g.bench_function("getset", |b| {
            b.iter(|| dispatch!(black_box(&store), "GETSET", "key:0", "newval"))
        });
    }

    {
        let store = populated_store(100);
        let mut i = 0usize;
        g.bench_function("getdel", |b| {
            b.iter(|| {
                let key = format!("key:{}", i % 100);
                i = i.wrapping_add(1);
                let args: &[CompactArg] = &[arg("SET"), arg(&key), arg("v")];
                dispatch_args(&store, args);
                let args2: &[CompactArg] = &[arg("GETDEL"), arg(&key)];
                dispatch_args(black_box(&store), args2)
            })
        });
    }

    {
        let store = populated_store(1);
        g.bench_function("setnx_existing", |b| {
            b.iter(|| dispatch!(black_box(&store), "SETNX", "key:0", "x"))
        });
    }

    {
        let store = Store::new(16);
        g.bench_function("setnx_new", |b| {
            b.iter(|| dispatch!(black_box(&store), "SETNX", "fresh", "x"))
        });
    }

    g.finish();
}

fn bench_string_multi(c: &mut Criterion) {
    let mut g = c.benchmark_group("string_multi");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    for n in [1usize, 10, 100] {
        let store = populated_store(n);

        g.bench_with_input(BenchmarkId::new("mget", n), &n, |b, &n| {
            let mut args: Vec<CompactArg> = vec![arg("MGET")];
            for i in 0..n {
                args.push(arg(&format!("key:{i}")));
            }
            b.iter(|| dispatch_args(black_box(&store), black_box(&args)))
        });

        g.bench_with_input(BenchmarkId::new("mset", n), &n, |b, &n| {
            let store2 = Store::new(16);
            let mut args: Vec<CompactArg> = vec![arg("MSET")];
            for i in 0..n {
                args.push(arg(&format!("key:{i}")));
                args.push(arg(&format!("val:{i}")));
            }
            b.iter(|| dispatch_args(black_box(&store2), black_box(&args)))
        });
    }

    g.finish();
}

fn bench_hash(c: &mut Criterion) {
    let mut g = c.benchmark_group("hash_core");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    {
        let store = Store::new(16);
        g.bench_function("hset_single", |b| {
            b.iter(|| {
                let args: &[CompactArg] =
                    &[arg("HSET"), arg("myhash"), arg("field1"), arg("value1")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = Store::new(16);
        let mut args: Vec<CompactArg> = vec![arg("HSET"), arg("myhash")];
        for i in 0..10 {
            args.push(arg(&format!("f{i}")));
            args.push(arg(&format!("v{i}")));
        }
        g.bench_function("hset_10_fields", |b| {
            b.iter(|| dispatch_args(black_box(&store), black_box(&args)))
        });
    }

    {
        let store = Store::new(16);
        let args: &[CompactArg] = &[arg("HSET"), arg("myhash"), arg("field1"), arg("value1")];
        dispatch_args(&store, args);
        g.bench_function("hget_hit", |b| {
            b.iter(|| {
                let args: &[CompactArg] = &[arg("HGET"), arg("myhash"), arg("field1")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = Store::new(16);
        let args: &[CompactArg] = &[arg("HSET"), arg("myhash"), arg("field1"), arg("value1")];
        dispatch_args(&store, args);
        g.bench_function("hget_miss", |b| {
            b.iter(|| {
                let args: &[CompactArg] = &[arg("HGET"), arg("myhash"), arg("no_field")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = Store::new(16);
        let mut hset_args: Vec<CompactArg> = vec![arg("HSET"), arg("myhash")];
        for i in 0..10 {
            hset_args.push(arg(&format!("f{i}")));
            hset_args.push(arg(&format!("v{i}")));
        }
        dispatch_args(&store, &hset_args);
        g.bench_function("hgetall_10", |b| {
            b.iter(|| dispatch!(black_box(&store), "HGETALL", "myhash"))
        });
    }

    {
        let store = Store::new(16);
        let mut hset_args: Vec<CompactArg> = vec![arg("HSET"), arg("myhash")];
        for i in 0..10 {
            hset_args.push(arg(&format!("f{i}")));
            hset_args.push(arg(&format!("v{i}")));
        }
        dispatch_args(&store, &hset_args);
        let hmget_args: Vec<CompactArg> = vec![
            arg("HMGET"),
            arg("myhash"),
            arg("f0"),
            arg("f2"),
            arg("f4"),
            arg("f6"),
            arg("f8"),
        ];
        g.bench_function("hmget_5", |b| {
            b.iter(|| dispatch_args(black_box(&store), black_box(&hmget_args)))
        });
    }

    {
        let store = Store::new(16);
        g.bench_function("hdel_single", |b| {
            b.iter(|| {
                let hset: &[CompactArg] = &[arg("HSET"), arg("myhash"), arg("f"), arg("v")];
                dispatch_args(&store, hset);
                let hdel: &[CompactArg] = &[arg("HDEL"), arg("myhash"), arg("f")];
                dispatch_args(black_box(&store), hdel)
            })
        });
    }

    g.finish();
}

fn bench_keyspace(c: &mut Criterion) {
    let mut g = c.benchmark_group("keyspace");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    {
        let store = populated_store(1);
        g.bench_function("del_hit", |b| {
            b.iter(|| {
                dispatch!(&store, "SET", "key:0", "v");
                dispatch!(black_box(&store), "DEL", "key:0")
            })
        });
    }

    {
        let store = Store::new(16);
        g.bench_function("del_miss", |b| {
            b.iter(|| dispatch!(black_box(&store), "DEL", "nope"))
        });
    }

    {
        let store = populated_store(1);
        g.bench_function("exists_hit", |b| {
            b.iter(|| dispatch!(black_box(&store), "EXISTS", "key:0"))
        });
        g.bench_function("exists_miss", |b| {
            b.iter(|| dispatch!(black_box(&store), "EXISTS", "nope"))
        });
    }

    for n in [1usize, 100, 1000] {
        let store = populated_store(n);
        g.bench_with_input(BenchmarkId::new("dbsize", n), &n, |b, _| {
            b.iter(|| dispatch!(black_box(&store), "DBSIZE"))
        });
    }

    g.finish();
}

fn bench_ttl(c: &mut Criterion) {
    let mut g = c.benchmark_group("ttl");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    {
        let store = populated_store(1);
        g.bench_function("expire_set", |b| {
            b.iter(|| {
                let args: &[CompactArg] = &[arg("EXPIRE"), arg("key:0"), arg("3600")];
                dispatch_args(black_box(&store), args)
            })
        });
    }

    {
        let store = populated_store(1);
        let args: &[CompactArg] = &[arg("EXPIRE"), arg("key:0"), arg("3600")];
        dispatch_args(&store, args);
        g.bench_function("ttl_read", |b| {
            b.iter(|| dispatch!(black_box(&store), "TTL", "key:0"))
        });
    }

    {
        let store = populated_store(1);
        g.bench_function("persist", |b| {
            b.iter(|| {
                let expire: &[CompactArg] = &[arg("EXPIRE"), arg("key:0"), arg("3600")];
                dispatch_args(&store, expire);
                dispatch!(black_box(&store), "PERSIST", "key:0")
            })
        });
    }

    g.finish();
}

fn bench_parse_i64(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_i64");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    let inputs: &[(&str, &[u8])] = &[
        ("zero", b"0"),
        ("small", b"42"),
        ("negative", b"-42"),
        ("large", b"9223372036854775807"),
        ("negative_large", b"-9223372036854775808"),
        ("invalid", b"not_a_number"),
    ];

    for (name, input) in inputs {
        g.bench_with_input(
            BenchmarkId::new("util_parse_i64", name),
            input,
            |b, input| b.iter(|| commands::util::parse_i64_bytes(black_box(input))),
        );
    }

    g.finish();
}

fn bench_helpers(c: &mut Criterion) {
    let mut g = c.benchmark_group("helpers");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    g.bench_function("wrong_args", |b| {
        b.iter(|| commands::util::wrong_args(black_box("HSET")))
    });

    g.bench_function("wrong_type", |b| b.iter(commands::util::wrong_type));

    g.bench_function("int_error", |b| b.iter(commands::util::int_error));

    g.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let mut g = c.benchmark_group("mixed_workload");
    g.warm_up_time(Duration::from_millis(500));
    g.measurement_time(Duration::from_millis(750));

    {
        let store = populated_store(1000);
        let keys: Vec<String> = (0..1000).map(|i| format!("key:{i}")).collect();
        let mut idx = 0usize;
        g.bench_function("80pct_get_20pct_set", |b| {
            b.iter(|| {
                let k = &keys[idx % 1000];
                idx = idx.wrapping_add(1);
                if idx.is_multiple_of(5) {
                    let args: &[CompactArg] = &[arg("SET"), arg(k), arg("newval")];
                    dispatch_args(black_box(&store), args)
                } else {
                    let args: &[CompactArg] = &[arg("GET"), arg(k)];
                    dispatch_args(black_box(&store), args)
                }
            })
        });
    }

    {
        let store = Store::new(16);
        dispatch!(&store, "SET", "ctr", "0");
        g.bench_function("pure_incr", |b| {
            b.iter(|| dispatch!(black_box(&store), "INCR", "ctr"))
        });
    }

    g.finish();
}

criterion_group!(
    benches,
    bench_dispatcher,
    bench_string_get_set,
    bench_string_multi,
    bench_hash,
    bench_keyspace,
    bench_ttl,
    bench_parse_i64,
    bench_helpers,
    bench_mixed_workload,
);
criterion_main!(benches);

use bytes::BytesMut;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use protocol::{
    encoder::Encoder,
    parser,
    types::{BulkData, RespFrame},
};
use std::hint::black_box;
use types::value::{CompactArg, CompactValue};

fn inline_command() -> BytesMut {
    BytesMut::from(&b"set session:42 Hello-World EX 300 NX\r\n"[..])
}

fn array_command(argc: usize, arg_len: usize) -> BytesMut {
    let mut out = Vec::new();
    out.extend_from_slice(format!("*{argc}\r\n").as_bytes());

    for index in 0..argc {
        let arg = format!("arg{index:02}_{}", "x".repeat(arg_len.saturating_sub(6)));
        out.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        out.extend_from_slice(arg.as_bytes());
        out.extend_from_slice(b"\r\n");
    }

    BytesMut::from(out.as_slice())
}

fn nested_frame() -> BytesMut {
    BytesMut::from(
        &b"*4\r\n+OK\r\n:42\r\n$11\r\nhello world\r\n*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"[..],
    )
}

fn map_like_frame() -> RespFrame {
    RespFrame::Map(vec![
        (
            RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(b"mode")))),
            RespFrame::SimpleStatic("active"),
        ),
        (
            RespFrame::Bulk(Some(BulkData::Arg(CompactArg::from_slice(b"count")))),
            RespFrame::Integer(128),
        ),
    ])
}

fn array_frame(items: usize, item_len: usize) -> RespFrame {
    let values = (0..items)
        .map(|index| {
            let value = format!(
                "value-{index:02}-{}",
                "x".repeat(item_len.saturating_sub(9))
            );
            RespFrame::Bulk(Some(BulkData::Value(CompactValue::from_vec(
                value.into_bytes(),
            ))))
        })
        .collect();
    RespFrame::Array(Some(values))
}

fn bulk_values_frame(items: usize, item_len: usize) -> RespFrame {
    let values = (0..items)
        .map(|index| {
            CompactValue::from_vec(
                format!(
                    "payload-{index:02}-{}",
                    "y".repeat(item_len.saturating_sub(11))
                )
                .into_bytes(),
            )
        })
        .collect();
    RespFrame::BulkValues(values)
}

fn bench_parse_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_command_into");

    let inline = inline_command();
    group.throughput(Throughput::Bytes(inline.len() as u64));
    group.bench_function("inline", |b| {
        b.iter_batched(
            || (inline.clone(), Vec::with_capacity(8)),
            |(mut src, mut args)| {
                parser::parse_command_into(black_box(&mut src), black_box(&mut args)).unwrap();
                black_box(args)
            },
            BatchSize::SmallInput,
        )
    });

    for &(argc, arg_len) in &[(3usize, 8usize), (8, 16), (32, 24)] {
        let src = array_command(argc, arg_len);
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("array", format!("{argc}x{arg_len}")),
            &src,
            |b, src| {
                b.iter_batched(
                    || (src.clone(), Vec::with_capacity(argc)),
                    |(mut src, mut args)| {
                        parser::parse_command_into(black_box(&mut src), black_box(&mut args))
                            .unwrap();
                        black_box(args)
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn bench_parse_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_frame");

    for (name, frame) in [
        ("simple", BytesMut::from(&b"+PONG\r\n"[..])),
        ("bulk", BytesMut::from(&b"$18\r\nbenchmark-payload\r\n"[..])),
        ("nested", nested_frame()),
    ] {
        group.throughput(Throughput::Bytes(frame.len() as u64));
        group.bench_function(name, |b| {
            b.iter_batched(
                || frame.clone(),
                |mut src| {
                    parser::parse_frame(black_box(&mut src)).unwrap();
                    black_box(src)
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    let cases = [
        ("simple", RespFrame::SimpleStatic("PONG")),
        ("bulk_values_8x16", bulk_values_frame(8, 16)),
        ("array_16x24", array_frame(16, 24)),
        ("map", map_like_frame()),
    ];

    for (name, frame) in cases {
        let mut warmup = BytesMut::new();
        Encoder::default().encode(&frame, &mut warmup);
        group.throughput(Throughput::Bytes(warmup.len() as u64));
        group.bench_function(name, |b| {
            let mut encoder = Encoder::default();
            b.iter(|| {
                let mut out = BytesMut::with_capacity(256);
                encoder.encode(black_box(&frame), black_box(&mut out));
                black_box(out)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_command,
    bench_parse_frame,
    bench_encode
);
criterion_main!(benches);

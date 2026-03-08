use super::{BenchKind, BenchSpec};

pub(crate) const TESTS: &[BenchSpec] = &[
    bench("ping_inline", "PING_INLINE", BenchKind::PingInline),
    bench("ping_mbulk", "PING_MBULK", BenchKind::PingMbulk),
    bench("set", "SET", BenchKind::Set),
    bench("get", "GET", BenchKind::Get),
    bench("incr", "INCR", BenchKind::Incr),
    bench("lpush", "LPUSH", BenchKind::Lpush),
    bench("rpush", "RPUSH", BenchKind::Rpush),
    bench("lpop", "LPOP", BenchKind::Lpop),
    bench("rpop", "RPOP", BenchKind::Rpop),
    bench("sadd", "SADD", BenchKind::Sadd),
    bench("hset", "HSET", BenchKind::Hset),
    bench("spop", "SPOP", BenchKind::Spop),
    bench("zadd", "ZADD", BenchKind::Zadd),
    bench("zpopmin", "ZPOPMIN", BenchKind::ZpopMin),
    bench("lrange_100", "LRANGE_100", BenchKind::Lrange100),
    bench("lrange_300", "LRANGE_300", BenchKind::Lrange300),
    bench("lrange_500", "LRANGE_500", BenchKind::Lrange500),
    bench("lrange_600", "LRANGE_600", BenchKind::Lrange600),
    bench("mset", "MSET", BenchKind::Mset),
];

pub fn tests() -> &'static [BenchSpec] {
    TESTS
}

pub(crate) fn find_test(input: &str) -> Option<BenchSpec> {
    let normalized = normalize_name(input);
    TESTS
        .iter()
        .copied()
        .find(|spec| normalize_name(spec.key) == normalized)
}

pub(crate) fn unknown_test_error(raw: &str) -> String {
    let supported = TESTS
        .iter()
        .map(|spec| spec.key)
        .collect::<Vec<_>>()
        .join(",");
    format!("unknown test '{raw}', supported tests include: {supported}")
}

fn normalize_name(input: &str) -> String {
    input
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

const fn bench(key: &'static str, name: &'static str, kind: BenchKind) -> BenchSpec {
    BenchSpec { key, name, kind }
}

#[derive(Clone, Copy, Debug)]
pub enum BenchKind {
    PingInline,
    PingMbulk,
    Set,
    Get,
    Incr,
    Lpush,
    Rpush,
    Sadd,
    Hset,
    Zadd,
}

#[derive(Clone, Copy, Debug)]
pub struct BenchSpec {
    pub name: &'static str,
    pub kind: BenchKind,
}

pub fn parse_specs(input: &[String]) -> Result<Vec<BenchSpec>, String> {
    if input.is_empty() {
        return Ok(default_specs());
    }

    let mut specs = Vec::new();
    for raw in input {
        let key = raw.trim().to_ascii_lowercase().replace([' ', '-', '_'], "");
        let spec = match key.as_str() {
            "ping" | "pinginline" => BenchSpec {
                name: "PING_INLINE",
                kind: BenchKind::PingInline,
            },
            "pingmbulk" => BenchSpec {
                name: "PING_MBULK",
                kind: BenchKind::PingMbulk,
            },
            "set" => BenchSpec {
                name: "SET",
                kind: BenchKind::Set,
            },
            "get" => BenchSpec {
                name: "GET",
                kind: BenchKind::Get,
            },
            "incr" => BenchSpec {
                name: "INCR",
                kind: BenchKind::Incr,
            },
            "lpush" => BenchSpec {
                name: "LPUSH",
                kind: BenchKind::Lpush,
            },
            "rpush" => BenchSpec {
                name: "RPUSH",
                kind: BenchKind::Rpush,
            },
            "sadd" => BenchSpec {
                name: "SADD",
                kind: BenchKind::Sadd,
            },
            "hset" => BenchSpec {
                name: "HSET",
                kind: BenchKind::Hset,
            },
            "zadd" => BenchSpec {
                name: "ZADD",
                kind: BenchKind::Zadd,
            },
            _ => {
                return Err(format!(
                    "unknown test '{raw}', supported tests: ping_inline,ping_mbulk,set,get,incr,lpush,rpush,sadd,hset,zadd"
                ));
            }
        };
        specs.push(spec);
    }
    Ok(specs)
}

fn default_specs() -> Vec<BenchSpec> {
    vec![
        BenchSpec {
            name: "PING_INLINE",
            kind: BenchKind::PingInline,
        },
        BenchSpec {
            name: "PING_MBULK",
            kind: BenchKind::PingMbulk,
        },
        BenchSpec {
            name: "SET",
            kind: BenchKind::Set,
        },
        BenchSpec {
            name: "GET",
            kind: BenchKind::Get,
        },
        BenchSpec {
            name: "INCR",
            kind: BenchKind::Incr,
        },
        BenchSpec {
            name: "LPUSH",
            kind: BenchKind::Lpush,
        },
        BenchSpec {
            name: "RPUSH",
            kind: BenchKind::Rpush,
        },
        BenchSpec {
            name: "SADD",
            kind: BenchKind::Sadd,
        },
        BenchSpec {
            name: "HSET",
            kind: BenchKind::Hset,
        },
        BenchSpec {
            name: "ZADD",
            kind: BenchKind::Zadd,
        },
    ]
}

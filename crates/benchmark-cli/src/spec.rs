#[derive(Clone, Copy, Debug)]
pub enum BenchKind {
    PingInline,
    PingMbulk,
    Echo,
    Set,
    SetNx,
    Get,
    GetSet,
    Mset,
    Mget,
    Del,
    Exists,
    Expire,
    Ttl,
    Incr,
    IncrBy,
    Decr,
    DecrBy,
    Strlen,
    SetRange,
    GetRange,
    Lpush,
    Rpush,
    Lpop,
    Rpop,
    Llen,
    Lrange,
    Sadd,
    Srem,
    Scard,
    Sismember,
    Hset,
    Hget,
    Hgetall,
    Hincrby,
    Zadd,
    Zrem,
    Zcard,
    Zscore,
    Zrank,
    Zrevrank,
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
            "echo" => BenchSpec {
                name: "ECHO",
                kind: BenchKind::Echo,
            },
            "set" => BenchSpec {
                name: "SET",
                kind: BenchKind::Set,
            },
            "setnx" => BenchSpec {
                name: "SETNX",
                kind: BenchKind::SetNx,
            },
            "get" => BenchSpec {
                name: "GET",
                kind: BenchKind::Get,
            },
            "getset" => BenchSpec {
                name: "GETSET",
                kind: BenchKind::GetSet,
            },
            "mset" => BenchSpec {
                name: "MSET",
                kind: BenchKind::Mset,
            },
            "mget" => BenchSpec {
                name: "MGET",
                kind: BenchKind::Mget,
            },
            "del" => BenchSpec {
                name: "DEL",
                kind: BenchKind::Del,
            },
            "exists" => BenchSpec {
                name: "EXISTS",
                kind: BenchKind::Exists,
            },
            "expire" => BenchSpec {
                name: "EXPIRE",
                kind: BenchKind::Expire,
            },
            "ttl" => BenchSpec {
                name: "TTL",
                kind: BenchKind::Ttl,
            },
            "incr" => BenchSpec {
                name: "INCR",
                kind: BenchKind::Incr,
            },
            "incrby" => BenchSpec {
                name: "INCRBY",
                kind: BenchKind::IncrBy,
            },
            "decr" => BenchSpec {
                name: "DECR",
                kind: BenchKind::Decr,
            },
            "decrby" => BenchSpec {
                name: "DECRBY",
                kind: BenchKind::DecrBy,
            },
            "strlen" => BenchSpec {
                name: "STRLEN",
                kind: BenchKind::Strlen,
            },
            "setrange" => BenchSpec {
                name: "SETRANGE",
                kind: BenchKind::SetRange,
            },
            "getrange" => BenchSpec {
                name: "GETRANGE",
                kind: BenchKind::GetRange,
            },
            "lpush" => BenchSpec {
                name: "LPUSH",
                kind: BenchKind::Lpush,
            },
            "rpush" => BenchSpec {
                name: "RPUSH",
                kind: BenchKind::Rpush,
            },
            "lpop" => BenchSpec {
                name: "LPOP",
                kind: BenchKind::Lpop,
            },
            "rpop" => BenchSpec {
                name: "RPOP",
                kind: BenchKind::Rpop,
            },
            "llen" => BenchSpec {
                name: "LLEN",
                kind: BenchKind::Llen,
            },
            "lrange" => BenchSpec {
                name: "LRANGE",
                kind: BenchKind::Lrange,
            },
            "sadd" => BenchSpec {
                name: "SADD",
                kind: BenchKind::Sadd,
            },
            "srem" => BenchSpec {
                name: "SREM",
                kind: BenchKind::Srem,
            },
            "scard" => BenchSpec {
                name: "SCARD",
                kind: BenchKind::Scard,
            },
            "sismember" => BenchSpec {
                name: "SISMEMBER",
                kind: BenchKind::Sismember,
            },
            "hset" => BenchSpec {
                name: "HSET",
                kind: BenchKind::Hset,
            },
            "hget" => BenchSpec {
                name: "HGET",
                kind: BenchKind::Hget,
            },
            "hgetall" => BenchSpec {
                name: "HGETALL",
                kind: BenchKind::Hgetall,
            },
            "hincrby" => BenchSpec {
                name: "HINCRBY",
                kind: BenchKind::Hincrby,
            },
            "zadd" => BenchSpec {
                name: "ZADD",
                kind: BenchKind::Zadd,
            },
            "zrem" => BenchSpec {
                name: "ZREM",
                kind: BenchKind::Zrem,
            },
            "zcard" => BenchSpec {
                name: "ZCARD",
                kind: BenchKind::Zcard,
            },
            "zscore" => BenchSpec {
                name: "ZSCORE",
                kind: BenchKind::Zscore,
            },
            "zrank" => BenchSpec {
                name: "ZRANK",
                kind: BenchKind::Zrank,
            },
            "zrevrank" => BenchSpec {
                name: "ZREVRANK",
                kind: BenchKind::Zrevrank,
            },
            _ => {
                return Err(format!(
                    "unknown test '{raw}', supported tests include: ping_inline,ping_mbulk,echo,set,setnx,get,getset,mset,mget,del,exists,expire,ttl,incr,incrby,decr,decrby,strlen,setrange,getrange,lpush,rpush,lpop,rpop,llen,lrange,sadd,srem,scard,sismember,hset,hget,hgetall,hincrby,zadd,zrem,zcard,zscore,zrank,zrevrank"
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
            name: "ECHO",
            kind: BenchKind::Echo,
        },
        BenchSpec {
            name: "SET",
            kind: BenchKind::Set,
        },
        BenchSpec {
            name: "SETNX",
            kind: BenchKind::SetNx,
        },
        BenchSpec {
            name: "GET",
            kind: BenchKind::Get,
        },
        BenchSpec {
            name: "GETSET",
            kind: BenchKind::GetSet,
        },
        BenchSpec {
            name: "MSET",
            kind: BenchKind::Mset,
        },
        BenchSpec {
            name: "MGET",
            kind: BenchKind::Mget,
        },
        BenchSpec {
            name: "DEL",
            kind: BenchKind::Del,
        },
        BenchSpec {
            name: "EXISTS",
            kind: BenchKind::Exists,
        },
        BenchSpec {
            name: "EXPIRE",
            kind: BenchKind::Expire,
        },
        BenchSpec {
            name: "TTL",
            kind: BenchKind::Ttl,
        },
        BenchSpec {
            name: "INCR",
            kind: BenchKind::Incr,
        },
        BenchSpec {
            name: "INCRBY",
            kind: BenchKind::IncrBy,
        },
        BenchSpec {
            name: "DECR",
            kind: BenchKind::Decr,
        },
        BenchSpec {
            name: "DECRBY",
            kind: BenchKind::DecrBy,
        },
        BenchSpec {
            name: "STRLEN",
            kind: BenchKind::Strlen,
        },
        BenchSpec {
            name: "SETRANGE",
            kind: BenchKind::SetRange,
        },
        BenchSpec {
            name: "GETRANGE",
            kind: BenchKind::GetRange,
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
            name: "LPOP",
            kind: BenchKind::Lpop,
        },
        BenchSpec {
            name: "RPOP",
            kind: BenchKind::Rpop,
        },
        BenchSpec {
            name: "LLEN",
            kind: BenchKind::Llen,
        },
        BenchSpec {
            name: "LRANGE",
            kind: BenchKind::Lrange,
        },
        BenchSpec {
            name: "SADD",
            kind: BenchKind::Sadd,
        },
        BenchSpec {
            name: "SREM",
            kind: BenchKind::Srem,
        },
        BenchSpec {
            name: "SCARD",
            kind: BenchKind::Scard,
        },
        BenchSpec {
            name: "SISMEMBER",
            kind: BenchKind::Sismember,
        },
        BenchSpec {
            name: "HSET",
            kind: BenchKind::Hset,
        },
        BenchSpec {
            name: "HGET",
            kind: BenchKind::Hget,
        },
        BenchSpec {
            name: "HGETALL",
            kind: BenchKind::Hgetall,
        },
        BenchSpec {
            name: "HINCRBY",
            kind: BenchKind::Hincrby,
        },
        BenchSpec {
            name: "ZADD",
            kind: BenchKind::Zadd,
        },
        BenchSpec {
            name: "ZREM",
            kind: BenchKind::Zrem,
        },
        BenchSpec {
            name: "ZCARD",
            kind: BenchKind::Zcard,
        },
        BenchSpec {
            name: "ZSCORE",
            kind: BenchKind::Zscore,
        },
        BenchSpec {
            name: "ZRANK",
            kind: BenchKind::Zrank,
        },
        BenchSpec {
            name: "ZREVRANK",
            kind: BenchKind::Zrevrank,
        },
    ]
}

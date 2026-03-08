use bytes::{BufMut, BytesMut};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use crate::cli::Config;

const RAND_TOKEN: &[u8] = b"__rand_int__";
const CRLF: &[u8] = b"\r\n";
const ZERO_TOKEN: [u8; 12] = *b"000000000000";

#[derive(Clone)]
pub struct BenchPlan {
    pub title: String,
    pub command: CommandTemplate,
    pub setup: Option<CommandTemplate>,
}

#[derive(Clone)]
pub enum CommandTemplate {
    Inline(Vec<u8>),
    Encoded(Vec<u8>),
    Resp(Vec<ArgTemplate>),
}

#[derive(Clone)]
pub enum ArgTemplate {
    Static(Vec<u8>),
    Payload,
    RandomToken(Vec<Vec<u8>>),
}

pub struct CommandState {
    payload: Vec<u8>,
    rng: StdRng,
    itoa: itoa::Buffer,
}

impl CommandState {
    pub fn new(seed: u64, data_size: usize) -> Self {
        Self {
            payload: vec![b'x'; data_size],
            rng: StdRng::seed_from_u64(seed),
            itoa: itoa::Buffer::new(),
        }
    }

    pub fn encode(
        &mut self,
        template: &CommandTemplate,
        keyspace_len: Option<u64>,
        out: &mut BytesMut,
    ) {
        match template {
            CommandTemplate::Inline(bytes) | CommandTemplate::Encoded(bytes) => {
                out.extend_from_slice(bytes)
            }
            CommandTemplate::Resp(args) => {
                out.put_u8(b'*');
                out.extend_from_slice(self.itoa.format(args.len()).as_bytes());
                out.extend_from_slice(CRLF);
                for arg in args {
                    self.encode_arg(arg, keyspace_len, out);
                }
            }
        }
    }

    fn encode_arg(&mut self, arg: &ArgTemplate, keyspace_len: Option<u64>, out: &mut BytesMut) {
        match arg {
            ArgTemplate::Static(value) => encode_bulk(value, &mut self.itoa, out),
            ArgTemplate::Payload => encode_bulk(&self.payload, &mut self.itoa, out),
            ArgTemplate::RandomToken(parts) => {
                let digits = random_digits(keyspace_len, &mut self.rng);
                let len = random_arg_len(parts, digits.len());
                out.put_u8(b'$');
                out.extend_from_slice(self.itoa.format(len).as_bytes());
                out.extend_from_slice(CRLF);
                for (index, part) in parts.iter().enumerate() {
                    out.extend_from_slice(part);
                    if index + 1 != parts.len() {
                        out.extend_from_slice(&digits);
                    }
                }
                out.extend_from_slice(CRLF);
            }
        }
    }
}

pub fn plans(config: &Config, stdin_payload: Option<Vec<u8>>) -> Vec<BenchPlan> {
    if !config.command.is_empty() {
        return vec![custom_plan(config, stdin_payload)];
    }

    let mut plans = vec![
        plan(
            "PING_INLINE",
            CommandTemplate::Inline(b"PING\r\n".to_vec()),
            None,
        ),
        plan("PING_MBULK", resp(config, [key("PING")]), None),
        plan(
            "SET",
            resp(config, [key("SET"), key("key:__rand_int__"), payload()]),
            None,
        ),
        plan(
            "GET",
            resp(config, [key("GET"), key("key:__rand_int__")]),
            None,
        ),
        plan(
            "INCR",
            resp(config, [key("INCR"), key("counter:__rand_int__")]),
            None,
        ),
        plan(
            "LPUSH",
            resp(config, [key("LPUSH"), key("list:__rand_int__"), payload()]),
            None,
        ),
        plan(
            "RPUSH",
            resp(config, [key("RPUSH"), key("list:__rand_int__"), payload()]),
            None,
        ),
        plan(
            "LPOP",
            resp(config, [key("LPOP"), key("list:__rand_int__")]),
            Some(resp(
                config,
                [key("LPUSH"), key("list:__rand_int__"), payload()],
            )),
        ),
        plan(
            "RPOP",
            resp(config, [key("RPOP"), key("list:__rand_int__")]),
            Some(resp(
                config,
                [key("RPUSH"), key("list:__rand_int__"), payload()],
            )),
        ),
        plan(
            "SADD",
            resp(
                config,
                [
                    key("SADD"),
                    key("set:__rand_int__"),
                    key("member:__rand_int__"),
                ],
            ),
            None,
        ),
        plan(
            "HSET",
            resp(
                config,
                [
                    key("HSET"),
                    key("hash:__rand_int__"),
                    key("field:__rand_int__"),
                    payload(),
                ],
            ),
            None,
        ),
        plan(
            "SPOP",
            resp(config, [key("SPOP"), key("set:__rand_int__")]),
            Some(resp(
                config,
                [
                    key("SADD"),
                    key("set:__rand_int__"),
                    key("member:__rand_int__"),
                ],
            )),
        ),
        plan(
            "ZADD",
            resp(
                config,
                [
                    key("ZADD"),
                    key("zset:__rand_int__"),
                    key("1"),
                    key("member:__rand_int__"),
                ],
            ),
            None,
        ),
        plan(
            "ZPOPMIN",
            resp(config, [key("ZPOPMIN"), key("zset:__rand_int__")]),
            Some(resp(
                config,
                [
                    key("ZADD"),
                    key("zset:__rand_int__"),
                    key("1"),
                    key("member:__rand_int__"),
                ],
            )),
        ),
        plan(
            "LPUSH (needed to benchmark LRANGE)",
            resp(config, [key("LPUSH"), key("lrange:key"), payload()]),
            None,
        ),
        plan(
            "LRANGE_100 (first 100 elements)",
            resp(
                config,
                [key("LRANGE"), key("lrange:key"), key("0"), key("99")],
            ),
            Some(resp(config, [key("LPUSH"), key("lrange:key"), payload()])),
        ),
        plan(
            "LRANGE_300 (first 300 elements)",
            resp(
                config,
                [key("LRANGE"), key("lrange:key"), key("0"), key("299")],
            ),
            Some(resp(config, [key("LPUSH"), key("lrange:key"), payload()])),
        ),
        plan(
            "LRANGE_500 (first 500 elements)",
            resp(
                config,
                [key("LRANGE"), key("lrange:key"), key("0"), key("499")],
            ),
            Some(resp(config, [key("LPUSH"), key("lrange:key"), payload()])),
        ),
        plan(
            "LRANGE_600 (first 600 elements)",
            resp(
                config,
                [key("LRANGE"), key("lrange:key"), key("0"), key("599")],
            ),
            Some(resp(config, [key("LPUSH"), key("lrange:key"), payload()])),
        ),
        plan(
            "MSET (10 keys)",
            resp(
                config,
                [
                    key("MSET"),
                    key("mset:1:__rand_int__"),
                    payload(),
                    key("mset:2:__rand_int__"),
                    payload(),
                    key("mset:3:__rand_int__"),
                    payload(),
                    key("mset:4:__rand_int__"),
                    payload(),
                    key("mset:5:__rand_int__"),
                    payload(),
                    key("mset:6:__rand_int__"),
                    payload(),
                    key("mset:7:__rand_int__"),
                    payload(),
                    key("mset:8:__rand_int__"),
                    payload(),
                    key("mset:9:__rand_int__"),
                    payload(),
                    key("mset:10:__rand_int__"),
                    payload(),
                ],
            ),
            None,
        ),
        plan(
            "XADD",
            resp(
                config,
                [
                    key("XADD"),
                    key("stream:__rand_int__"),
                    key("*"),
                    key("field"),
                    payload(),
                ],
            ),
            None,
        ),
    ];

    if let Some(selected) = &config.tests {
        plans.retain(|plan| selected.iter().any(|name| matches_test(name, &plan.title)));
    }

    plans
}

fn plan(title: &str, command: CommandTemplate, setup: Option<CommandTemplate>) -> BenchPlan {
    BenchPlan {
        title: title.to_string(),
        command,
        setup,
    }
}

fn custom_plan(config: &Config, stdin_payload: Option<Vec<u8>>) -> BenchPlan {
    let mut args = Vec::with_capacity(config.command.len());
    for (index, value) in config.command.iter().enumerate() {
        if config.stdin_last_arg && index + 1 == config.command.len() {
            args.push(ArgTemplate::Static(
                stdin_payload.clone().unwrap_or_default(),
            ));
        } else {
            args.push(split_random(value.as_bytes()));
        }
    }

    BenchPlan {
        title: config.command.join(" "),
        command: compile(config, args),
        setup: None,
    }
}

fn resp<const N: usize>(config: &Config, args: [ArgTemplate; N]) -> CommandTemplate {
    compile(config, args.into_iter().collect())
}

fn key(value: &str) -> ArgTemplate {
    split_random(value.as_bytes())
}

fn payload() -> ArgTemplate {
    ArgTemplate::Payload
}

fn split_random(raw: &[u8]) -> ArgTemplate {
    if !raw
        .windows(RAND_TOKEN.len())
        .any(|window| window == RAND_TOKEN)
    {
        return ArgTemplate::Static(raw.to_vec());
    }

    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(pos) = raw[start..]
        .windows(RAND_TOKEN.len())
        .position(|window| window == RAND_TOKEN)
    {
        let split = start + pos;
        parts.push(raw[start..split].to_vec());
        start = split + RAND_TOKEN.len();
    }
    parts.push(raw[start..].to_vec());
    ArgTemplate::RandomToken(parts)
}

fn compile(config: &Config, args: Vec<ArgTemplate>) -> CommandTemplate {
    if config.keyspace_len.is_some() && has_random(&args) {
        return CommandTemplate::Resp(args);
    }

    let mut out = BytesMut::with_capacity(estimate_size(&args, config.data_size));
    let mut itoa = itoa::Buffer::new();
    let payload = vec![b'x'; config.data_size];

    out.put_u8(b'*');
    out.extend_from_slice(itoa.format(args.len()).as_bytes());
    out.extend_from_slice(CRLF);

    for arg in &args {
        match arg {
            ArgTemplate::Static(value) => encode_bulk(value, &mut itoa, &mut out),
            ArgTemplate::Payload => encode_bulk(&payload, &mut itoa, &mut out),
            ArgTemplate::RandomToken(parts) => {
                encode_random(parts, ZERO_TOKEN, &mut itoa, &mut out)
            }
        }
    }

    CommandTemplate::Encoded(out.to_vec())
}

fn has_random(args: &[ArgTemplate]) -> bool {
    args.iter()
        .any(|arg| matches!(arg, ArgTemplate::RandomToken(_)))
}

fn estimate_size(args: &[ArgTemplate], data_size: usize) -> usize {
    16 + args
        .iter()
        .map(|arg| match arg {
            ArgTemplate::Static(value) => value.len() + 16,
            ArgTemplate::Payload => data_size + 16,
            ArgTemplate::RandomToken(parts) => random_arg_len(parts, ZERO_TOKEN.len()) + 16,
        })
        .sum::<usize>()
}

fn encode_bulk(value: &[u8], itoa: &mut itoa::Buffer, out: &mut BytesMut) {
    out.put_u8(b'$');
    out.extend_from_slice(itoa.format(value.len()).as_bytes());
    out.extend_from_slice(CRLF);
    out.extend_from_slice(value);
    out.extend_from_slice(CRLF);
}

fn encode_random(parts: &[Vec<u8>], digits: [u8; 12], itoa: &mut itoa::Buffer, out: &mut BytesMut) {
    out.put_u8(b'$');
    out.extend_from_slice(itoa.format(random_arg_len(parts, digits.len())).as_bytes());
    out.extend_from_slice(CRLF);
    for (index, part) in parts.iter().enumerate() {
        out.extend_from_slice(part);
        if index + 1 != parts.len() {
            out.extend_from_slice(&digits);
        }
    }
    out.extend_from_slice(CRLF);
}

fn random_arg_len(parts: &[Vec<u8>], digit_len: usize) -> usize {
    parts.iter().map(Vec::len).sum::<usize>() + parts.len().saturating_sub(1) * digit_len
}

fn random_digits(keyspace_len: Option<u64>, rng: &mut StdRng) -> [u8; 12] {
    let mut out = ZERO_TOKEN;
    let mut value = keyspace_len
        .map(|limit| rng.random_range(0..limit))
        .unwrap_or(0);
    let mut index = out.len();
    while value != 0 && index != 0 {
        index -= 1;
        out[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    out
}

fn matches_test(name: &str, title: &str) -> bool {
    if name.eq_ignore_ascii_case("ping") && title.starts_with("PING_") {
        return true;
    }
    let base = title.split_once(' ').map_or(title, |(head, _)| head);
    name.eq_ignore_ascii_case(&base.to_ascii_lowercase()) || name.eq_ignore_ascii_case(base)
}

use crate::resp::{encode_expected_response, encode_resp_parts, ExpectedResponse};
use crate::workload::{ArgTemplate, BenchKind, BenchRun, CommandTemplate};

use super::model::{RandomSource, RequestGroup};

const LIST_ITEM_COUNT: usize = 600;
const MSET_KEYS: usize = 10;

pub fn build_request_group(
    run: &BenchRun,
    key_base: &[u8],
    value: &[u8],
    batch: usize,
    random: &mut RandomSource,
) -> Result<RequestGroup, String> {
    let mut payload = Vec::new();
    let mut expected = Vec::with_capacity(batch);
    let mut encoded = Vec::with_capacity(batch);

    for _ in 0..batch {
        let slot = pick_key_slot(random, run.random_keyspace_len);
        let frame = build_command(run, key_base, slot, value, random)?;
        payload.extend_from_slice(&frame);

        let expected_response = expected_response(run.kind, value);
        encoded.push(
            expected_response
                .as_ref()
                .and_then(encode_expected_response),
        );
        expected.push(expected_response);
    }

    Ok(RequestGroup {
        payload,
        uniform_encoded: shared_uniform_encoded(&encoded),
        expected,
        encoded,
    })
}

pub fn build_setup_command(
    kind: BenchKind,
    key_base: &[u8],
    slot: u64,
    value: &[u8],
) -> Option<Vec<u8>> {
    let key = make_key(key_base, slot);
    match kind {
        BenchKind::Get => Some(encode_resp_parts(&[b"SET", key.as_slice(), value])),
        BenchKind::Lpop | BenchKind::Rpop => {
            Some(encode_resp_parts(&[b"LPUSH", key.as_slice(), value]))
        }
        BenchKind::Spop => Some(encode_resp_parts(&[b"SADD", key.as_slice(), value])),
        BenchKind::ZpopMin => Some(encode_resp_parts(&[b"ZADD", key.as_slice(), b"1", value])),
        BenchKind::Lrange100
        | BenchKind::Lrange300
        | BenchKind::Lrange500
        | BenchKind::Lrange600 => build_lrange_setup(&key, kind),
        _ => None,
    }
}

pub fn build_mset_command(key_base: &[u8], slot: u64, value: &[u8]) -> Vec<u8> {
    let mut owned = Vec::with_capacity(MSET_KEYS * 2);
    let mut parts = Vec::with_capacity(1 + MSET_KEYS * 2);
    parts.push(b"MSET".as_slice());
    for index in 0..MSET_KEYS {
        let key = format!("{}:{}:{}", String::from_utf8_lossy(key_base), slot, index).into_bytes();
        owned.push(key);
        owned.push(value.to_vec());
    }
    for item in &owned {
        parts.push(item.as_slice());
    }
    encode_resp_parts(&parts)
}

pub fn make_key(base: &[u8], slot: u64) -> Vec<u8> {
    if slot == 0 {
        return base.to_vec();
    }
    let mut key = Vec::with_capacity(base.len() + 24);
    key.extend_from_slice(base);
    key.push(b':');
    key.extend_from_slice(slot.to_string().as_bytes());
    key
}

fn build_command(
    run: &BenchRun,
    key_base: &[u8],
    slot: u64,
    value: &[u8],
    random: &mut RandomSource,
) -> Result<Vec<u8>, String> {
    if run.kind != BenchKind::PingInline {
        if let Some(template) = run.command.as_ref() {
            return build_custom_command(
                template,
                key_base,
                slot,
                value,
                random,
                run.random_keyspace_len,
            );
        }
    }

    Ok(match run.kind {
        BenchKind::PingInline => b"PING\r\n".to_vec(),
        BenchKind::PingMbulk
        | BenchKind::Set
        | BenchKind::Get
        | BenchKind::Incr
        | BenchKind::Lpush
        | BenchKind::Rpush
        | BenchKind::Lpop
        | BenchKind::Rpop
        | BenchKind::Sadd
        | BenchKind::Hset
        | BenchKind::Spop
        | BenchKind::Zadd
        | BenchKind::ZpopMin
        | BenchKind::Lrange100
        | BenchKind::Lrange300
        | BenchKind::Lrange500
        | BenchKind::Lrange600
        | BenchKind::Mset
        | BenchKind::Custom => unreachable!("command template missing for benchmark"),
    })
}

fn build_custom_command(
    template: &CommandTemplate,
    key_base: &[u8],
    slot: u64,
    value: &[u8],
    random: &mut RandomSource,
    keyspace: Option<u64>,
) -> Result<Vec<u8>, String> {
    let mut parts = Vec::with_capacity(template.parts.len());
    let mut owned = Vec::with_capacity(template.parts.len());
    let key = make_key(key_base, slot);
    for part in &template.parts {
        match part {
            ArgTemplate::Literal(value) => owned.push(value.clone()),
            ArgTemplate::Key => owned.push(key.clone()),
            ArgTemplate::KeySuffix(suffix) => {
                let mut derived = key.clone();
                derived.extend_from_slice(suffix);
                owned.push(derived);
            }
            ArgTemplate::Data => owned.push(value.to_vec()),
            ArgTemplate::RandomInt => {
                let range =
                    keyspace.ok_or_else(|| "__rand_int__ requires -r <keyspacelen>".to_string())?;
                owned.push((random.next() % range).to_string().into_bytes());
            }
        }
    }

    for item in &owned {
        parts.push(item.as_slice());
    }
    Ok(encode_resp_parts(&parts))
}

fn expected_response(kind: BenchKind, value: &[u8]) -> Option<ExpectedResponse> {
    match kind {
        BenchKind::PingInline | BenchKind::PingMbulk => Some(ExpectedResponse::Simple("PONG")),
        BenchKind::Set | BenchKind::Mset => Some(ExpectedResponse::Simple("OK")),
        BenchKind::Get | BenchKind::Lpop | BenchKind::Rpop => {
            Some(ExpectedResponse::Bulk(Some(value.to_vec())))
        }
        BenchKind::Incr => None,
        BenchKind::Lpush
        | BenchKind::Rpush
        | BenchKind::Sadd
        | BenchKind::Hset
        | BenchKind::Zadd => None,
        BenchKind::Spop => Some(ExpectedResponse::Bulk(Some(value.to_vec()))),
        BenchKind::ZpopMin => Some(ExpectedResponse::Array(vec![
            ExpectedResponse::Bulk(Some(value.to_vec())),
            ExpectedResponse::Bulk(Some(b"1".to_vec())),
        ])),
        BenchKind::Lrange100
        | BenchKind::Lrange300
        | BenchKind::Lrange500
        | BenchKind::Lrange600 => None,
        BenchKind::Custom => None,
    }
}

fn build_lrange_setup(key: &[u8], kind: BenchKind) -> Option<Vec<u8>> {
    let mut parts = Vec::with_capacity(2 + LIST_ITEM_COUNT);
    parts.push(b"LPUSH".as_slice());
    parts.push(key);

    let target = lrange_target(kind);
    let mut items = Vec::with_capacity(target);
    for index in 0..target {
        items.push(format!("item:{index}").into_bytes());
    }
    for item in &items {
        parts.push(item.as_slice());
    }

    Some(encode_resp_parts(&parts))
}

fn lrange_target(kind: BenchKind) -> usize {
    match kind {
        BenchKind::Lrange100 => 100,
        BenchKind::Lrange300 => 300,
        BenchKind::Lrange500 => 500,
        BenchKind::Lrange600 => 600,
        _ => LIST_ITEM_COUNT,
    }
}

fn pick_key_slot(random: &mut RandomSource, keyspace: Option<u64>) -> u64 {
    match keyspace {
        Some(0) | None => 0,
        Some(1) => 0,
        Some(keyspace) => random.next() % keyspace,
    }
}

fn shared_uniform_encoded(encoded: &[Option<Vec<u8>>]) -> Option<Vec<u8>> {
    let first = encoded.first()?.as_ref()?;
    encoded
        .iter()
        .all(|item| item.as_ref().is_some_and(|bytes| bytes == first))
        .then(|| first.clone())
}

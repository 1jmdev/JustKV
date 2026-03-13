mod blocking;
mod core;
mod moves;
mod range;

use crate::util::{eq_ascii, int_error, parse_u64_bytes};
use engine::store::ListSide;
use protocol::types::{BulkData, RespFrame};

pub(crate) use blocking::{blmpop, blpop, brpop};
pub(crate) use core::{llen, lpop, lpush, lpushx, lrem, rpop, rpush, rpushx};
pub(crate) use moves::{brpoplpush, lmove, lmpop, rpoplpush};
pub(crate) use range::{lindex, linsert, lpos, lrange, lset, ltrim};

fn parse_side(raw: &[u8]) -> Result<ListSide, RespFrame> {
    if eq_ascii(raw, b"LEFT") {
        Ok(ListSide::Left)
    } else if eq_ascii(raw, b"RIGHT") {
        Ok(ListSide::Right)
    } else {
        Err(crate::util::syntax_error())
    }
}

fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(value).map_err(|_| int_error())
}

fn parse_timeout(raw: &[u8]) -> Result<f64, ()> {
    std::str::from_utf8(raw)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .ok_or(())
}

fn lmpop_response(
    key: types::value::CompactKey,
    values: Vec<types::value::CompactValue>,
) -> RespFrame {
    RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::Arg(key))),
        RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| RespFrame::Bulk(Some(BulkData::Value(value))))
                .collect(),
        )),
    ]))
}

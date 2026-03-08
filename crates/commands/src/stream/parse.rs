use crate::util::{Args, int_error, parse_u64_bytes};
use engine::store::{XAddId, XTrimMode};
use protocol::types::RespFrame;
use types::value::StreamId;

const ERR_STREAM_ID: &str = "ERR Invalid stream ID specified";

pub(super) fn parse_stream_id(raw: &[u8]) -> Result<StreamId, RespFrame> {
    if raw == b"-" {
        return Ok(StreamId { ms: 0, seq: 0 });
    }
    if raw == b"+" {
        return Ok(StreamId {
            ms: u64::MAX,
            seq: u64::MAX,
        });
    }
    if raw == b"$" {
        return Ok(StreamId {
            ms: u64::MAX,
            seq: u64::MAX,
        });
    }
    let Some(dash) = memchr::memchr(b'-', raw) else {
        let ms = parse_u64_bytes(raw).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
        return Ok(StreamId { ms, seq: 0 });
    };
    let ms = parse_u64_bytes(&raw[..dash]).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
    let seq =
        parse_u64_bytes(&raw[dash + 1..]).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
    Ok(StreamId { ms, seq })
}

pub(super) fn parse_xadd_id(raw: &[u8]) -> Result<XAddId, RespFrame> {
    if raw == b"*" {
        return Ok(XAddId::Auto);
    }
    let dash = memchr::memchr(b'-', raw).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
    let ms = parse_u64_bytes(&raw[..dash]).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
    let seq_bytes = &raw[dash + 1..];
    if seq_bytes == b"*" {
        return Ok(XAddId::AutoSeqAtMs { ms });
    }
    let seq = parse_u64_bytes(seq_bytes).ok_or_else(|| RespFrame::error_static(ERR_STREAM_ID))?;
    Ok(XAddId::Explicit { ms, seq })
}

pub(super) fn parse_count(raw: &[u8]) -> Result<usize, RespFrame> {
    let v = parse_u64_bytes(raw).ok_or_else(int_error)?;
    usize::try_from(v).map_err(|_| int_error())
}

pub(super) fn parse_xtrim_args(
    args: &Args,
    mut index: usize,
) -> Result<(Option<(XTrimMode, StreamId, Option<usize>)>, usize), RespFrame> {
    let _trace = profiler::scope("commands::stream::parse::parse_xtrim_args");
    if index >= args.len() || !args[index].eq_ignore_ascii_case(b"MAXLEN") {
        return Ok((None, index));
    }
    index += 1;
    if index >= args.len() {
        return Err(crate::util::syntax_error());
    }
    if args[index].as_slice() == b"~" || args[index].as_slice() == b"=" {
        index += 1;
    }
    if index >= args.len() {
        return Err(crate::util::syntax_error());
    }
    let max_len = parse_count(&args[index])?;
    index += 1;
    Ok((
        Some((
            XTrimMode::MaxLen,
            StreamId {
                ms: max_len as u64,
                seq: 0,
            },
            None,
        )),
        index,
    ))
}

pub(super) fn stream_id_to_bulk(id: StreamId) -> RespFrame {
    let _trace = profiler::scope("commands::stream::parse::stream_id_to_bulk");
    RespFrame::Bulk(Some(protocol::types::BulkData::from_vec(
        format!("{}-{}", id.ms, id.seq).into_bytes(),
    )))
}

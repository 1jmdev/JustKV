use crate::commands::util::{Args, int_error};
use crate::engine::store::{XAddId, XTrimMode};
use crate::engine::value::StreamId;
use crate::protocol::types::RespFrame;

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
    let text = std::str::from_utf8(raw)
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    let Some((ms, seq)) = text.split_once('-') else {
        return Err(RespFrame::Error(
            "ERR Invalid stream ID specified".to_string(),
        ));
    };
    let ms = ms
        .parse::<u64>()
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    let seq = seq
        .parse::<u64>()
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    Ok(StreamId { ms, seq })
}

pub(super) fn parse_xadd_id(raw: &[u8]) -> Result<XAddId, RespFrame> {
    if raw == b"*" {
        return Ok(XAddId::Auto);
    }
    let text = std::str::from_utf8(raw)
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    let Some((ms, seq)) = text.split_once('-') else {
        return Err(RespFrame::Error(
            "ERR Invalid stream ID specified".to_string(),
        ));
    };
    let ms = ms
        .parse::<u64>()
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    if seq == "*" {
        return Ok(XAddId::AutoSeqAtMs { ms });
    }
    let seq = seq
        .parse::<u64>()
        .map_err(|_| RespFrame::Error("ERR Invalid stream ID specified".to_string()))?;
    Ok(XAddId::Explicit { ms, seq })
}

pub(super) fn parse_count(raw: &[u8]) -> Result<usize, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

pub(super) fn parse_xtrim_args(
    args: &Args,
    mut index: usize,
) -> Result<(Option<(XTrimMode, StreamId, Option<usize>)>, usize), RespFrame> {
    if index >= args.len() || !args[index].eq_ignore_ascii_case(b"MAXLEN") {
        return Ok((None, index));
    }
    index += 1;
    if index >= args.len() {
        return Err(RespFrame::Error("ERR syntax error".to_string()));
    }
    if args[index].as_slice() == b"~" || args[index].as_slice() == b"=" {
        index += 1;
    }
    if index >= args.len() {
        return Err(RespFrame::Error("ERR syntax error".to_string()));
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
    RespFrame::Bulk(Some(crate::protocol::types::BulkData::from_vec(
        format!("{}-{}", id.ms, id.seq).into_bytes(),
    )))
}

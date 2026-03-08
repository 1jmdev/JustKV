use bytes::{Buf, BytesMut};

pub(crate) fn try_skip_frame(buf: &mut BytesMut) -> Result<Option<()>, String> {
    let Some(consumed) = frame_len(buf.as_ref(), 0)? else {
        return Ok(None);
    };
    buf.advance(consumed);
    Ok(Some(()))
}

fn frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    if start >= src.len() {
        return Ok(None);
    }

    match src[start] {
        b'+' | b'-' | b':' => line_frame_len(src, start),
        b'$' => bulk_frame_len(src, start),
        b'*' => aggregate_frame_len(src, start, 1),
        b'%' => aggregate_frame_len(src, start, 2),
        other => Err(format!("unsupported RESP type byte: {other:?}")),
    }
}

fn line_frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    Ok(Some(end + 2 - start))
}

fn bulk_frame_len(src: &[u8], start: usize) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    let len = parse_i64_ascii(&src[start + 1..end])?;
    if len < 0 {
        return Ok(Some(end + 2 - start));
    }
    let total = end + 2 + len as usize + 2;
    if src.len() < total {
        return Ok(None);
    }
    Ok(Some(total - start))
}

fn aggregate_frame_len(
    src: &[u8],
    start: usize,
    multiplier: usize,
) -> Result<Option<usize>, String> {
    let Some(end) = find_crlf(src, start + 1) else {
        return Ok(None);
    };
    let count = parse_i64_ascii(&src[start + 1..end])?;
    if count < 0 {
        return Ok(Some(end + 2 - start));
    }

    let mut cursor = end + 2;
    for _ in 0..(count as usize * multiplier) {
        let Some(len) = frame_len(src, cursor)? else {
            return Ok(None);
        };
        cursor += len;
    }
    Ok(Some(cursor - start))
}

fn find_crlf(src: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    while index + 1 < src.len() {
        if src[index] == b'\r' && src[index + 1] == b'\n' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_i64_ascii(raw: &[u8]) -> Result<i64, String> {
    let text = std::str::from_utf8(raw).map_err(|err| format!("invalid integer bytes: {err}"))?;
    text.parse::<i64>()
        .map_err(|err| format!("invalid integer {text:?}: {err}"))
}

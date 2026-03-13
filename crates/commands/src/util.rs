use bytes::{BufMut, BytesMut};
use protocol::types::{BulkData, RespFrame};
use types::value::CompactArg;

pub type Args = [CompactArg];

pub struct ScanOptions<'a> {
    pub cursor: u64,
    pub pattern: Option<&'a [u8]>,
    pub count: usize,
}

#[inline]
pub fn eq_ascii(command: &[u8], expected: &[u8]) -> bool {
    command == expected || command.eq_ignore_ascii_case(expected)
}

#[cold]
pub fn wrong_args(command: &str) -> RespFrame {
    let command = command.to_ascii_lowercase();
    RespFrame::Error(format!(
        "ERR wrong number of arguments for '{command}' command"
    ))
}

#[inline(always)]
pub fn syntax_error() -> RespFrame {
    RespFrame::error_static("ERR syntax error")
}

#[inline(always)]
pub fn invalid_cursor() -> RespFrame {
    RespFrame::error_static("ERR invalid cursor")
}

#[inline(always)]
pub fn timeout_error() -> RespFrame {
    RespFrame::error_static("ERR timeout is not a float or out of range")
}

#[inline(always)]
pub fn int_error() -> RespFrame {
    RespFrame::error_static("ERR value is not an integer or out of range")
}

#[inline(always)]
pub fn wrong_type() -> RespFrame {
    RespFrame::error_static("WRONGTYPE Operation against a key holding the wrong kind of value")
}

pub fn u64_to_bytes(value: u64) -> Vec<u8> {
    let mut buffer = itoa::Buffer::new();
    buffer.format(value).as_bytes().to_vec()
}

pub fn f64_to_bytes(value: f64) -> Vec<u8> {
    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format(value);
    if let Some(trimmed) = formatted.strip_suffix(".0") {
        trimmed.as_bytes().to_vec()
    } else {
        formatted.as_bytes().to_vec()
    }
}

pub fn parse_i64_bytes(raw: &[u8]) -> Option<i64> {
    if raw.is_empty() {
        return None;
    }

    let mut index = 0;
    let negative;
    match raw[0] {
        b'-' => {
            negative = true;
            index = 1;
        }
        b'+' => {
            negative = false;
            index = 1;
        }
        _ => {
            negative = false;
        }
    }

    let digits = &raw[index..];
    let n = digits.len();
    if n == 0 || n > 19 {
        return None;
    }

    // For ≤18 digits the value fits in u64 unconditionally (max 10^18-1 < u64::MAX),
    // so we skip checked arithmetic entirely.
    if n <= 18 {
        let mut value: u64 = 0;
        for &b in digits {
            let d = b.wrapping_sub(b'0');
            if d > 9 {
                return None;
            }
            value = value * 10 + u64::from(d);
        }
        let v = value as i64;
        if negative {
            Some(v.wrapping_neg())
        } else {
            Some(v)
        }
    } else {
        // 19-digit path: checked arithmetic to catch overflow.
        let mut value: i64 = 0;
        for &b in digits {
            let d = b.wrapping_sub(b'0');
            if d > 9 {
                return None;
            }
            value = value.checked_mul(10)?.checked_add(i64::from(d))?;
        }
        if negative {
            value.checked_neg()
        } else {
            Some(value)
        }
    }
}

#[inline]
pub fn parse_u64_bytes(raw: &[u8]) -> Option<u64> {
    let n = raw.len();
    if n == 0 || n > 20 {
        return None;
    }
    // ≤19 digits always fit in u64 (max 10^19-1 < u64::MAX = 1.8×10^19).
    if n <= 19 {
        let mut value: u64 = 0;
        for &b in raw {
            let d = b.wrapping_sub(b'0');
            if d > 9 {
                return None;
            }
            value = value * 10 + u64::from(d);
        }
        Some(value)
    } else {
        // 20-digit path: need overflow check (u64::MAX is 20 digits).
        let mut value: u64 = 0;
        for &b in raw {
            let d = b.wrapping_sub(b'0');
            if d > 9 {
                return None;
            }
            value = value.checked_mul(10)?.checked_add(u64::from(d))?;
        }
        Some(value)
    }
}

pub fn parse_scan_options<'a>(args: &'a Args, command: &str) -> Result<ScanOptions<'a>, RespFrame> {
    if args.len() < 3 {
        return Err(wrong_args(command));
    }

    let cursor = parse_u64_bytes(args[2].as_slice()).ok_or_else(invalid_cursor)?;
    let mut pattern = None;
    let mut count = 10usize;
    let mut index = 3;
    while index < args.len() {
        if eq_ascii(&args[index], b"MATCH") {
            index += 1;
            if index >= args.len() {
                return Err(syntax_error());
            }
            pattern = Some(args[index].as_slice());
        } else if eq_ascii(&args[index], b"COUNT") {
            index += 1;
            if index >= args.len() {
                return Err(syntax_error());
            }
            let value = parse_u64_bytes(args[index].as_slice()).ok_or_else(invalid_cursor)?;
            count = usize::try_from(value).map_err(|_| int_error())?;
        } else {
            return Err(syntax_error());
        }
        index += 1;
    }

    Ok(ScanOptions {
        cursor,
        pattern,
        count,
    })
}

pub fn scan_array_response(next_cursor: u64, items: Vec<RespFrame>) -> RespFrame {
    RespFrame::Array(Some(vec![
        RespFrame::Bulk(Some(BulkData::from_vec(u64_to_bytes(next_cursor)))),
        RespFrame::Array(Some(items)),
    ]))
}

pub fn preencode_bulk_str(value: &str) -> bytes::Bytes {
    let mut len_buf = itoa::Buffer::new();
    let mut out = BytesMut::with_capacity(1 + 20 + 2 + value.len() + 2);
    out.put_u8(b'$');
    out.put_slice(len_buf.format(value.len()).as_bytes());
    out.put_slice(b"\r\n");
    out.put_slice(value.as_bytes());
    out.put_slice(b"\r\n");
    out.freeze()
}

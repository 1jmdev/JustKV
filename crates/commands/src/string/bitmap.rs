use crate::util::{Args, int_error, wrong_args, wrong_type};
use engine::store::{BitFieldEncoding, BitFieldOp, BitFieldOverflow, BitOp, Store};
use protocol::types::RespFrame;

pub(crate) fn getbit(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 3 {
        return wrong_args("GETBIT");
    }
    let offset = match parse_non_negative_usize(&args[2], int_error()) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.getbit(&args[1], offset) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn setbit(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 4 {
        return wrong_args("SETBIT");
    }
    let offset = match parse_non_negative_usize(&args[2], int_error()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let bit = match parse_bit(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    match store.setbit(&args[1], offset, bit) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bitcount(store: &Store, args: &Args) -> RespFrame {
    if args.len() != 2 && args.len() != 4 && args.len() != 5 {
        return wrong_args("BITCOUNT");
    }

    let mut start = None;
    let mut end = None;
    let mut bit_unit = false;

    if args.len() >= 4 {
        start = Some(match parse_i64(&args[2]) {
            Ok(value) => value,
            Err(response) => return response,
        });
        end = Some(match parse_i64(&args[3]) {
            Ok(value) => value,
            Err(response) => return response,
        });
    }
    if args.len() == 5 {
        bit_unit = match parse_index_unit(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        };
    }

    match store.bitcount(&args[1], start, end, bit_unit) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bitpos(store: &Store, args: &Args) -> RespFrame {
    if !(3..=6).contains(&args.len()) {
        return wrong_args("BITPOS");
    }

    let bit = match parse_bit(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let start = if args.len() >= 4 {
        Some(match parse_i64(&args[3]) {
            Ok(value) => value,
            Err(response) => return response,
        })
    } else {
        None
    };
    let end = if args.len() >= 5 {
        Some(match parse_i64(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        })
    } else {
        None
    };
    let bit_unit = if args.len() == 6 {
        match parse_index_unit(&args[5]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        false
    };

    match store.bitpos(&args[1], bit, start, end, bit_unit) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bitop(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 4 {
        return wrong_args("BITOP");
    }

    let op = if args[1].eq_ignore_ascii_case(b"AND") {
        BitOp::And
    } else if args[1].eq_ignore_ascii_case(b"OR") {
        BitOp::Or
    } else if args[1].eq_ignore_ascii_case(b"XOR") {
        BitOp::Xor
    } else if args[1].eq_ignore_ascii_case(b"NOT") {
        BitOp::Not
    } else {
        return RespFrame::Error("ERR syntax error".to_string());
    };

    let sources = &args[3..];
    if matches!(op, BitOp::Not) && sources.len() != 1 {
        return RespFrame::Error(
            "ERR BITOP NOT must be called with a single source key.".to_string(),
        );
    }

    match store.bitop(op, &args[2], sources) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn bitfield(store: &Store, args: &Args) -> RespFrame {
    bitfield_impl(store, args, false)
}

pub(crate) fn bitfield_ro(store: &Store, args: &Args) -> RespFrame {
    bitfield_impl(store, args, true)
}

fn bitfield_impl(store: &Store, args: &Args, read_only: bool) -> RespFrame {
    if args.len() < 2 {
        return wrong_args(if read_only { "BITFIELD_RO" } else { "BITFIELD" });
    }

    let mut overflow = BitFieldOverflow::Wrap;
    let mut index = 2usize;
    let mut operations = Vec::new();

    while index < args.len() {
        let token = args[index].as_slice();
        if token.eq_ignore_ascii_case(b"OVERFLOW") {
            if read_only {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            index += 1;
            if index >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            overflow = if args[index].eq_ignore_ascii_case(b"WRAP") {
                BitFieldOverflow::Wrap
            } else if args[index].eq_ignore_ascii_case(b"SAT") {
                BitFieldOverflow::Sat
            } else if args[index].eq_ignore_ascii_case(b"FAIL") {
                BitFieldOverflow::Fail
            } else {
                return RespFrame::Error("ERR syntax error".to_string());
            };
            index += 1;
            continue;
        }

        if token.eq_ignore_ascii_case(b"GET") {
            if index + 2 >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            let encoding = match parse_bitfield_encoding(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let offset = match parse_bitfield_offset(&args[index + 2], encoding) {
                Ok(value) => value,
                Err(response) => return response,
            };
            operations.push(BitFieldOp::Get { encoding, offset });
            index += 3;
            continue;
        }

        if token.eq_ignore_ascii_case(b"SET") {
            if read_only {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            if index + 3 >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            let encoding = match parse_bitfield_encoding(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let offset = match parse_bitfield_offset(&args[index + 2], encoding) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let value = match parse_i64(&args[index + 3]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            operations.push(BitFieldOp::Set {
                encoding,
                offset,
                value,
            });
            index += 4;
            continue;
        }

        if token.eq_ignore_ascii_case(b"INCRBY") {
            if read_only {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            if index + 3 >= args.len() {
                return RespFrame::Error("ERR syntax error".to_string());
            }
            let encoding = match parse_bitfield_encoding(&args[index + 1]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let offset = match parse_bitfield_offset(&args[index + 2], encoding) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let increment = match parse_i64(&args[index + 3]) {
                Ok(value) => value,
                Err(response) => return response,
            };
            operations.push(BitFieldOp::IncrBy {
                encoding,
                offset,
                increment,
                overflow,
            });
            index += 4;
            continue;
        }

        return RespFrame::Error("ERR syntax error".to_string());
    }

    if operations.is_empty() {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    match store.bitfield(&args[1], &operations) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| match value {
                    Some(value) => RespFrame::Integer(value),
                    None => RespFrame::Bulk(None),
                })
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

fn parse_i64(raw: &[u8]) -> Result<i64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<i64>().map_err(|_| int_error()),
        Err(_) => Err(int_error()),
    }
}

fn parse_non_negative_usize(raw: &[u8], err: RespFrame) -> Result<usize, RespFrame> {
    let value = match std::str::from_utf8(raw) {
        Ok(value) => value.parse::<u64>().map_err(|_| err.clone())?,
        Err(_) => return Err(err),
    };
    usize::try_from(value).map_err(|_| int_error())
}

fn parse_bit(raw: &[u8]) -> Result<u8, RespFrame> {
    match raw {
        b"0" => Ok(0),
        b"1" => Ok(1),
        _ => Err(RespFrame::Error(
            "ERR bit is not an integer or out of range".to_string(),
        )),
    }
}

fn parse_index_unit(raw: &[u8]) -> Result<bool, RespFrame> {
    if raw.eq_ignore_ascii_case(b"BYTE") {
        Ok(false)
    } else if raw.eq_ignore_ascii_case(b"BIT") {
        Ok(true)
    } else {
        Err(RespFrame::Error("ERR syntax error".to_string()))
    }
}

fn parse_bitfield_encoding(raw: &[u8]) -> Result<BitFieldEncoding, RespFrame> {
    if raw.len() < 2 {
        return Err(RespFrame::Error("ERR invalid bitfield type".to_string()));
    }

    let bits = std::str::from_utf8(&raw[1..])
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| RespFrame::Error("ERR invalid bitfield type".to_string()))?;

    match raw[0].to_ascii_lowercase() {
        b'i' if (1..=64).contains(&bits) => Ok(BitFieldEncoding::Signed { bits }),
        b'u' if (1..=63).contains(&bits) => Ok(BitFieldEncoding::Unsigned { bits }),
        _ => Err(RespFrame::Error("ERR invalid bitfield type".to_string())),
    }
}

fn parse_bitfield_offset(raw: &[u8], encoding: BitFieldEncoding) -> Result<usize, RespFrame> {
    if let Some(rest) = raw.strip_prefix(b"#") {
        let stride = match encoding {
            BitFieldEncoding::Signed { bits } | BitFieldEncoding::Unsigned { bits } => bits as u64,
        };
        let index = parse_non_negative_usize(
            rest,
            RespFrame::Error("ERR bit offset is not an integer or out of range".to_string()),
        )? as u64;
        return usize::try_from(index.saturating_mul(stride)).map_err(|_| {
            RespFrame::Error("ERR bit offset is not an integer or out of range".to_string())
        });
    }

    parse_non_negative_usize(
        raw,
        RespFrame::Error("ERR bit offset is not an integer or out of range".to_string()),
    )
}

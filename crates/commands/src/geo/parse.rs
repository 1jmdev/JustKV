use crate::util::{Args, int_error};
use engine::store::Store;
use protocol::types::RespFrame;

#[derive(Clone, Copy)]
pub(super) enum SortOrder {
    Asc,
    Desc,
}

pub(super) struct SearchOptions {
    pub withcoord: bool,
    pub withdist: bool,
    pub withhash: bool,
    pub sort: Option<SortOrder>,
    pub count: Option<usize>,
    pub any: bool,
    pub store: Option<Vec<u8>>,
    pub storedist: Option<Vec<u8>>,
}

impl SearchOptions {
    pub(super) fn new() -> Self {
        Self {
            withcoord: false,
            withdist: false,
            withhash: false,
            sort: None,
            count: None,
            any: false,
            store: None,
            storedist: None,
        }
    }
}

pub(super) fn parse_f64(raw: &[u8]) -> Result<f64, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| RespFrame::Error("ERR value is not a valid float".to_string())),
        Err(_) => Err(RespFrame::Error(
            "ERR value is not a valid float".to_string(),
        )),
    }
}

pub(super) fn parse_usize(raw: &[u8]) -> Result<usize, RespFrame> {
    match std::str::from_utf8(raw) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| int_error())
            .and_then(|value| usize::try_from(value).map_err(|_| int_error())),
        Err(_) => Err(int_error()),
    }
}

pub(super) fn parse_distance_unit(raw: &[u8]) -> Result<f64, RespFrame> {
    if raw.eq_ignore_ascii_case(b"M") {
        Ok(1.0)
    } else if raw.eq_ignore_ascii_case(b"KM") {
        Ok(1000.0)
    } else if raw.eq_ignore_ascii_case(b"MI") {
        Ok(1609.344)
    } else if raw.eq_ignore_ascii_case(b"FT") {
        Ok(0.3048)
    } else {
        Err(RespFrame::Error(
            "ERR unsupported unit provided. please use M, KM, FT, MI".to_string(),
        ))
    }
}

pub(super) fn parse_search_options(
    args: &Args,
    mut index: usize,
) -> Result<SearchOptions, RespFrame> {
    let mut options = SearchOptions::new();
    while index < args.len() {
        let token = args[index].as_slice();
        if token.eq_ignore_ascii_case(b"WITHCOORD") {
            options.withcoord = true;
            index += 1;
        } else if token.eq_ignore_ascii_case(b"WITHDIST") {
            options.withdist = true;
            index += 1;
        } else if token.eq_ignore_ascii_case(b"WITHHASH") {
            options.withhash = true;
            index += 1;
        } else if token.eq_ignore_ascii_case(b"ASC") {
            options.sort = Some(SortOrder::Asc);
            index += 1;
        } else if token.eq_ignore_ascii_case(b"DESC") {
            options.sort = Some(SortOrder::Desc);
            index += 1;
        } else if token.eq_ignore_ascii_case(b"COUNT") {
            if index + 1 >= args.len() {
                return Err(RespFrame::Error("ERR syntax error".to_string()));
            }
            options.count = Some(parse_usize(&args[index + 1])?);
            index += 2;
            if index < args.len() && args[index].eq_ignore_ascii_case(b"ANY") {
                options.any = true;
                index += 1;
            }
        } else if token.eq_ignore_ascii_case(b"STORE") {
            if index + 1 >= args.len() {
                return Err(RespFrame::Error("ERR syntax error".to_string()));
            }
            options.store = Some(args[index + 1].to_vec());
            index += 2;
        } else if token.eq_ignore_ascii_case(b"STOREDIST") {
            if index + 1 >= args.len() {
                return Err(RespFrame::Error("ERR syntax error".to_string()));
            }
            options.storedist = Some(args[index + 1].to_vec());
            index += 2;
        } else {
            return Err(RespFrame::Error("ERR syntax error".to_string()));
        }
    }
    Ok(options)
}

pub(super) fn geosearch_center_from_member(
    store: &Store,
    key: &[u8],
    member: &[u8],
) -> Result<Option<(f64, f64)>, RespFrame> {
    let members = [engine::value::CompactArg::from_slice(member)];
    store
        .geopos(key, &members)
        .map_err(|_| crate::util::wrong_type())
        .map(|positions| positions.into_iter().next().flatten())
}

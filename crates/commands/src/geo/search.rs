use crate::geo::parse::{
    SearchOptions, SortOrder, parse_distance_unit, parse_f64, parse_search_options,
};
use crate::util::{Args, f64_to_bytes, wrong_args, wrong_type};
use engine::store::{GeoSearchMatch, Store};
use protocol::types::{BulkData, RespFrame};

pub(crate) fn geosearch(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::geo::search::geosearch");
    if args.len() < 7 {
        return wrong_args("GEOSEARCH");
    }

    let (center, shape_index) = if args[2].eq_ignore_ascii_case(b"FROMLONLAT") {
        let lon = match parse_f64(&args[3]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let lat = match parse_f64(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        ((lon, lat), 5)
    } else if args[2].eq_ignore_ascii_case(b"FROMMEMBER") {
        let center = match super::parse::geosearch_center_from_member(store, &args[1], &args[3]) {
            Ok(Some(value)) => value,
            Ok(None) => return RespFrame::Array(Some(vec![])),
            Err(response) => return response,
        };
        (center, 4)
    } else {
        return crate::util::syntax_error();
    };

    let (radius, box_size, index) = match parse_shape(args, shape_index) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let options = match parse_search_options(args, index) {
        Ok(value) => value,
        Err(response) => return response,
    };

    run_search(store, &args[1], center, radius, box_size, options)
}

pub(crate) fn geosearchstore(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::geo::search::geosearchstore");
    if args.len() < 8 {
        return wrong_args("GEOSEARCHSTORE");
    }

    let (center, shape_index) = if args[3].eq_ignore_ascii_case(b"FROMLONLAT") {
        let lon = match parse_f64(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let lat = match parse_f64(&args[5]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        ((lon, lat), 6)
    } else if args[3].eq_ignore_ascii_case(b"FROMMEMBER") {
        let center = match super::parse::geosearch_center_from_member(store, &args[2], &args[4]) {
            Ok(Some(value)) => value,
            Ok(None) => return RespFrame::Integer(0),
            Err(response) => return response,
        };
        (center, 5)
    } else {
        return crate::util::syntax_error();
    };

    let (radius, box_size, mut index) = match parse_shape(args, shape_index) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let mut options = SearchOptions::new();
    let mut store_dist = false;
    while index < args.len() {
        let token = args[index].as_slice();
        if token.eq_ignore_ascii_case(b"ASC") {
            options.sort = Some(SortOrder::Asc);
            index += 1;
        } else if token.eq_ignore_ascii_case(b"DESC") {
            options.sort = Some(SortOrder::Desc);
            index += 1;
        } else if token.eq_ignore_ascii_case(b"COUNT") {
            if index + 1 >= args.len() {
                return crate::util::syntax_error();
            }
            options.count = match super::parse::parse_usize(&args[index + 1]) {
                Ok(value) => Some(value),
                Err(response) => return response,
            };
            index += 2;
            if index < args.len() && args[index].eq_ignore_ascii_case(b"ANY") {
                options.any = true;
                index += 1;
            }
        } else if token.eq_ignore_ascii_case(b"STOREDIST") {
            store_dist = true;
            index += 1;
        } else {
            return crate::util::syntax_error();
        }
    }

    let ascending = !matches!(options.sort, Some(SortOrder::Desc));
    match store.geosearchstore(
        &args[1],
        &args[2],
        center,
        radius,
        box_size,
        ascending,
        options.count,
        store_dist,
    ) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn run_radius_search(
    store: &Store,
    key: &[u8],
    center: (f64, f64),
    radius_meters: f64,
    options: SearchOptions,
) -> RespFrame {
    let _trace = profiler::scope("commands::geo::search::run_radius_search");
    run_search(store, key, center, Some(radius_meters), None, options)
}

fn run_search(
    store: &Store,
    key: &[u8],
    center: (f64, f64),
    radius: Option<f64>,
    box_size: Option<(f64, f64)>,
    options: SearchOptions,
) -> RespFrame {
    let _trace = profiler::scope("commands::geo::search::run_search");
    if let Some(destination) = options.storedist.clone() {
        let ascending = !matches!(options.sort, Some(SortOrder::Desc));
        return match store.geosearchstore(
            &destination,
            key,
            center,
            radius,
            box_size,
            ascending,
            options.count,
            true,
        ) {
            Ok(value) => RespFrame::Integer(value),
            Err(_) => wrong_type(),
        };
    }
    if let Some(destination) = options.store.clone() {
        let ascending = !matches!(options.sort, Some(SortOrder::Desc));
        return match store.geosearchstore(
            &destination,
            key,
            center,
            radius,
            box_size,
            ascending,
            options.count,
            false,
        ) {
            Ok(value) => RespFrame::Integer(value),
            Err(_) => wrong_type(),
        };
    }

    let ascending = !matches!(options.sort, Some(SortOrder::Desc));
    match store.geosearch(key, center, radius, box_size, ascending, options.count) {
        Ok(matches) => format_matches(matches, options),
        Err(_) => wrong_type(),
    }
}

fn parse_shape(
    args: &Args,
    index: usize,
) -> Result<(Option<f64>, Option<(f64, f64)>, usize), RespFrame> {
    let _trace = profiler::scope("commands::geo::search::parse_shape");
    if index >= args.len() {
        return Err(crate::util::syntax_error());
    }
    if args[index].eq_ignore_ascii_case(b"BYRADIUS") {
        if index + 2 >= args.len() {
            return Err(crate::util::syntax_error());
        }
        let value = parse_f64(&args[index + 1])?;
        let unit = parse_distance_unit(&args[index + 2])?;
        return Ok((Some(value * unit), None, index + 3));
    }
    if args[index].eq_ignore_ascii_case(b"BYBOX") {
        if index + 3 >= args.len() {
            return Err(crate::util::syntax_error());
        }
        let width = parse_f64(&args[index + 1])?;
        let height = parse_f64(&args[index + 2])?;
        let unit = parse_distance_unit(&args[index + 3])?;
        return Ok((None, Some((width * unit, height * unit)), index + 4));
    }
    Err(crate::util::syntax_error())
}

fn format_matches(matches: Vec<GeoSearchMatch>, options: SearchOptions) -> RespFrame {
    let _trace = profiler::scope("commands::geo::search::format_matches");
    RespFrame::Array(Some(
        matches
            .into_iter()
            .map(|entry| {
                if !options.withcoord && !options.withdist && !options.withhash {
                    return RespFrame::Bulk(Some(BulkData::Arg(entry.member)));
                }

                let mut item = vec![RespFrame::Bulk(Some(BulkData::Arg(entry.member)))];
                if options.withdist {
                    item.push(RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(
                        entry.distance_meters.unwrap_or(0.0),
                    )))));
                }
                if options.withhash {
                    item.push(RespFrame::Integer(0));
                }
                if options.withcoord {
                    item.push(RespFrame::Array(Some(vec![
                        RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(entry.longitude)))),
                        RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(entry.latitude)))),
                    ])));
                }
                RespFrame::Array(Some(item))
            })
            .collect(),
    ))
}

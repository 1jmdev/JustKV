use crate::geo::parse::{parse_distance_unit, parse_f64};
use crate::util::{Args, f64_to_bytes, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::{BulkData, RespFrame};

pub(crate) fn geopos(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::geopos");
    if args.len() < 3 {
        return wrong_args("GEOPOS");
    }

    match store.geopos(&args[1], &args[2..]) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| match value {
                    Some((lon, lat)) => RespFrame::Array(Some(vec![
                        RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(lon)))),
                        RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(lat)))),
                    ])),
                    None => RespFrame::Bulk(None),
                })
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn geohash(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::geohash");
    if args.len() < 3 {
        return wrong_args("GEOHASH");
    }

    match store.geohash(&args[1], &args[2..]) {
        Ok(values) => RespFrame::Array(Some(
            values
                .into_iter()
                .map(|value| {
                    RespFrame::Bulk(value.map(|hash| BulkData::from_vec(hash.into_bytes())))
                })
                .collect(),
        )),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn geodist(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::geodist");
    if args.len() != 4 && args.len() != 5 {
        return wrong_args("GEODIST");
    }

    let multiplier = if args.len() == 5 {
        match parse_distance_unit(&args[4]) {
            Ok(value) => value,
            Err(response) => return response,
        }
    } else {
        1.0
    };

    match store.geodist(&args[1], &args[2], &args[3]) {
        Ok(Some(value)) => {
            RespFrame::Bulk(Some(BulkData::from_vec(f64_to_bytes(value / multiplier))))
        }
        Ok(None) => RespFrame::Bulk(None),
        Err(_) => wrong_type(),
    }
}

pub(crate) fn georadiusbymember(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::georadiusbymember");
    if args.len() < 5 {
        return wrong_args("GEORADIUSBYMEMBER");
    }

    let center = match super::parse::geosearch_center_from_member(store, &args[1], &args[2]) {
        Ok(Some(value)) => value,
        Ok(None) => return RespFrame::Array(Some(vec![])),
        Err(response) => return response,
    };
    geosearch_like_radius(store, args, center, 3, false)
}

pub(crate) fn georadiusbymember_ro(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::georadiusbymember_ro");
    if args.len() < 5 {
        return wrong_args("GEORADIUSBYMEMBER_RO");
    }

    let center = match super::parse::geosearch_center_from_member(store, &args[1], &args[2]) {
        Ok(Some(value)) => value,
        Ok(None) => return RespFrame::Array(Some(vec![])),
        Err(response) => return response,
    };
    geosearch_like_radius(store, args, center, 3, true)
}

pub(crate) fn georadius(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::georadius");
    if args.len() < 6 {
        return wrong_args("GEORADIUS");
    }
    let lon = match parse_f64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let lat = match parse_f64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    geosearch_like_radius(store, args, (lon, lat), 4, false)
}

pub(crate) fn georadius_ro(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::georadius_ro");
    if args.len() < 6 {
        return wrong_args("GEORADIUS_RO");
    }
    let lon = match parse_f64(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let lat = match parse_f64(&args[3]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    geosearch_like_radius(store, args, (lon, lat), 4, true)
}

fn geosearch_like_radius(
    store: &Store,
    args: &Args,
    center: (f64, f64),
    radius_index: usize,
    read_only: bool,
) -> RespFrame {
    let _trace = profiler::scope("crates::commands::src::geo::read::geosearch_like_radius");
    let radius = match parse_f64(&args[radius_index]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let unit = match parse_distance_unit(&args[radius_index + 1]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let options = match super::parse::parse_search_options(args, radius_index + 2) {
        Ok(value) => value,
        Err(response) => return response,
    };

    if read_only && (options.store.is_some() || options.storedist.is_some()) {
        return RespFrame::Error("ERR syntax error".to_string());
    }

    super::search::run_radius_search(store, &args[1], center, radius * unit, options)
}

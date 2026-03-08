use crate::geo::parse::parse_f64;
use crate::util::{Args, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn geoadd(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::geo::add::geoadd");
    if args.len() < 5 {
        return wrong_args("GEOADD");
    }

    let mut index = 2usize;
    let mut ch = false;
    while index < args.len() {
        if args[index].eq_ignore_ascii_case(b"CH") {
            ch = true;
            index += 1;
            continue;
        }
        break;
    }
    if args.len() <= index || (args.len() - index) % 3 != 0 {
        return wrong_args("GEOADD");
    }

    let mut items = Vec::with_capacity((args.len() - index) / 3);
    let mut changed = 0i64;
    for chunk in args[index..].chunks(3) {
        let lon = match parse_f64(&chunk[0]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        let lat = match parse_f64(&chunk[1]) {
            Ok(value) => value,
            Err(response) => return response,
        };
        if !(-180.0..=180.0).contains(&lon) || !(-85.051_128_78..=85.051_128_78).contains(&lat) {
            return RespFrame::Error("ERR invalid longitude,latitude pair".to_string());
        }
        if ch {
            match store.geopos(&args[1], &[chunk[2].clone()]) {
                Ok(existing) => {
                    if existing
                        .first()
                        .and_then(|value| *value)
                        .is_none_or(|(old_lon, old_lat)| old_lon != lon || old_lat != lat)
                    {
                        changed += 1;
                    }
                }
                Err(_) => return wrong_type(),
            }
        }
        items.push((lon, lat, chunk[2].clone()));
    }

    match store.geoadd(&args[1], &items) {
        Ok(value) => RespFrame::Integer(if ch { changed } else { value }),
        Err(_) => wrong_type(),
    }
}

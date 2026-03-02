use crate::geo::parse::parse_f64;
use crate::util::{Args, wrong_args, wrong_type};
use engine::store::Store;
use protocol::types::RespFrame;

pub(crate) fn geoadd(store: &Store, args: &Args) -> RespFrame {
    if args.len() < 5 || (args.len() - 2) % 3 != 0 {
        return wrong_args("GEOADD");
    }

    let mut items = Vec::with_capacity((args.len() - 2) / 3);
    for chunk in args[2..].chunks(3) {
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
        items.push((lon, lat, chunk[2].clone()));
    }

    match store.geoadd(&args[1], &items) {
        Ok(value) => RespFrame::Integer(value),
        Err(_) => wrong_type(),
    }
}

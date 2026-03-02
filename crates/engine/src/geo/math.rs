pub(super) fn haversine_meters(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let _trace = profiler::scope("engine::geo::math::haversine_meters");
    let earth_radius = 6_372_797.560_856_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    earth_radius * c
}

pub(super) fn meters_per_lat(delta_deg: f64) -> f64 {
    let _trace = profiler::scope("engine::geo::math::meters_per_lat");
    delta_deg * 111_132.0
}

pub(super) fn meters_per_lon(delta_deg: f64, lat_deg: f64) -> f64 {
    let _trace = profiler::scope("engine::geo::math::meters_per_lon");
    delta_deg * 111_320.0 * lat_deg.to_radians().cos().abs().max(0.000001)
}

pub(super) fn geohash11(lon: f64, lat: f64) -> String {
    let _trace = profiler::scope("engine::geo::math::geohash11");
    const BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";
    let mut lon_min = -180.0;
    let mut lon_max = 180.0;
    let mut lat_min = -90.0;
    let mut lat_max = 90.0;

    let mut hash = String::with_capacity(11);
    let mut bits = 0u8;
    let mut bit_count = 0u8;
    let mut use_lon = true;

    while hash.len() < 11 {
        if use_lon {
            let mid = (lon_min + lon_max) / 2.0;
            if lon >= mid {
                bits = (bits << 1) | 1;
                lon_min = mid;
            } else {
                bits <<= 1;
                lon_max = mid;
            }
        } else {
            let mid = (lat_min + lat_max) / 2.0;
            if lat >= mid {
                bits = (bits << 1) | 1;
                lat_min = mid;
            } else {
                bits <<= 1;
                lat_max = mid;
            }
        }
        use_lon = !use_lon;
        bit_count += 1;

        if bit_count == 5 {
            hash.push(BASE32[bits as usize] as char);
            bits = 0;
            bit_count = 0;
        }
    }

    hash
}

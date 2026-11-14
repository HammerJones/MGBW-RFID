/// Great-circle distance in kilometres.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0088_f64;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Rough bounds of NWS coverage for ski purposes: the 50 states.
pub fn in_us_bounds(lat: f64, lon: f64) -> bool {
    let conus = (24.0..=49.5).contains(&lat) && (-125.0..=-66.5).contains(&lon);
    let alaska = (51.0..=71.5).contains(&lat) && (-170.0..=-129.0).contains(&lon);
    let hawaii = (18.5..=22.5).contains(&lat) && (-160.5..=-154.5).contains(&lon);
    conus || alaska || hawaii
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denver_to_vail_distance() {
        let d = haversine_km(39.7392, -104.9903, 39.6403, -106.3742);
        assert!((117.0..=125.0).contains(&d), "{d}");
    }

    #[test]
    fn bounds() {
        assert!(in_us_bounds(39.64, -106.37));
        assert!(in_us_bounds(60.97, -149.1)); // Alyeska
        assert!(!in_us_bounds(50.11, -122.95)); // Whistler
        assert!(!in_us_bounds(0.0, 0.0));
        assert!(!in_us_bounds(f64::NAN, -100.0));
    }
}

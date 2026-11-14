//! NOAA MRMS base-reflectivity radar mosaic, served as WMS by NCEP.

use crate::http::Http;

const WMS: &str = "https://opengeo.ncep.noaa.gov/geoserver/conus/conus_bref_qcd/ows";
pub const MAX_FRAMES: usize = 8;

/// A MapLibre raster-tile URL template. `time` is one of `radar_times`,
/// or `None` for the latest frame.
pub fn tile_template(time: Option<&str>) -> String {
    let mut url = format!(
        "{WMS}?service=WMS&version=1.1.1&request=GetMap&layers=conus_bref_qcd&styles=&format=image/png&transparent=true&srs=EPSG:3857&width=256&height=256&bbox={{bbox-epsg-3857}}"
    );
    if let Some(t) = time.filter(|t| is_time(t)) {
        url.push_str("&time=");
        url.push_str(t);
    }
    url
}

fn is_time(s: &str) -> bool {
    s.len() >= 20 && s.len() <= 30 && s.ends_with('Z') && s.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '.'))
}

/// Recent frame times (oldest first), thinned to at most `MAX_FRAMES`.
pub fn fetch_times(http: &dyn Http) -> Result<Vec<String>, String> {
    let body = http
        .get(&format!("{WMS}?service=WMS&version=1.3.0&request=GetCapabilities"))
        .map_err(|e| format!("Radar times failed: {e}"))?;
    Ok(thin(parse_times(&body), MAX_FRAMES))
}

pub fn parse_times(caps: &str) -> Vec<String> {
    let Some(pos) = caps.find("<Dimension name=\"time\"") else { return Vec::new() };
    let rest = &caps[pos..];
    let (Some(open), Some(close)) = (rest.find('>'), rest.find("</Dimension>")) else { return Vec::new() };
    if close <= open {
        return Vec::new();
    }
    rest[open + 1..close].split(',').map(str::trim).filter(|t| is_time(t)).map(str::to_string).collect()
}

/// Evenly pick `max` entries, always keeping the newest.
pub fn thin(times: Vec<String>, max: usize) -> Vec<String> {
    if times.len() <= max || max < 2 {
        return times;
    }
    let last = times.len() - 1;
    (0..max).map(|i| times[i * last / (max - 1)].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAPS: &str = r#"<Layer><Name>conus_bref_qcd</Name><Dimension name="time" default="2026-11-11T09:43:58Z" units="ISO8601">2026-11-11T07:46:13.000Z,2026-11-11T07:47:58.000Z, 2026-11-11T09:43:58.000Z,<bad></Dimension></Layer>"#;

    #[test]
    fn parses_time_dimension() {
        assert_eq!(parse_times(CAPS), vec!["2026-11-11T07:46:13.000Z", "2026-11-11T07:47:58.000Z", "2026-11-11T09:43:58.000Z"]);
        assert!(parse_times("<xml/>").is_empty());
        assert!(parse_times("<Dimension name=\"time\"").is_empty());
    }

    #[test]
    fn thins_keeping_first_and_last() {
        let t: Vec<String> = (0..58).map(|i| format!("t{i}")).collect();
        let out = thin(t.clone(), 8);
        assert_eq!(out.len(), 8);
        assert_eq!(out[0], "t0");
        assert_eq!(out[7], "t57");
        assert_eq!(thin(t[..3].to_vec(), 8).len(), 3);
    }

    #[test]
    fn template_has_bbox_placeholder_and_optional_time() {
        let t = tile_template(None);
        assert!(t.ends_with("bbox={bbox-epsg-3857}"));
        assert!(t.contains("srs=EPSG:3857"));
        assert!(tile_template(Some("2026-11-11T09:43:58.000Z")).ends_with("&time=2026-11-11T09:43:58.000Z"));
        // Anything that is not a timestamp is ignored rather than injected into the URL.
        assert_eq!(tile_template(Some("x&layers=evil")), t);
    }
}

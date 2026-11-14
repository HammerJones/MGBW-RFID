//! National Weather Service API (api.weather.gov): gridded forecast,
//! active alerts and the forecaster's written discussion.

use crate::http::Http;
use crate::interval::{overlap_fraction, parse_valid_time, split_by_local_day};
use crate::model::{Alert, DaySnow};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const BASE: &str = "https://api.weather.gov";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointInfo {
    pub office: String,
    pub grid_url: String,
    pub radar_station: Option<String>,
    pub tz: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: DateTime<Utc>,
    pub dur: Duration,
    pub value: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grid {
    /// Snowfall per interval, centimetres.
    pub snow_cm: Vec<Span>,
    /// Temperature, °C.
    pub temp_c: Vec<Span>,
}

fn json(body: &str, what: &str) -> Result<Value, String> {
    serde_json::from_str(body).map_err(|e| format!("{what}: unreadable response ({e})"))
}

pub fn fetch_point(http: &dyn Http, lat: f64, lon: f64) -> Result<PointInfo, String> {
    // The API rejects more than four decimal places.
    let url = format!("{BASE}/points/{lat:.4},{lon:.4}");
    let body = http.get(&url).map_err(|e| format!("NWS location lookup failed: {e}"))?;
    parse_point(&body)
}

pub fn parse_point(body: &str) -> Result<PointInfo, String> {
    let v = json(body, "NWS location lookup")?;
    let p = &v["properties"];
    let office = p["gridId"].as_str().or_else(|| p["cwa"].as_str());
    let grid_url = p["forecastGridData"].as_str();
    match (office, grid_url) {
        (Some(office), Some(grid_url)) => Ok(PointInfo {
            office: office.to_string(),
            grid_url: grid_url.to_string(),
            radar_station: p["radarStation"].as_str().map(str::to_string),
            tz: p["timeZone"].as_str().unwrap_or("America/Denver").to_string(),
        }),
        _ => Err("NWS has no forecast grid for this location".to_string()),
    }
}

pub fn fetch_grid(http: &dyn Http, point: &PointInfo) -> Result<Grid, String> {
    let body = http.get(&point.grid_url).map_err(|e| format!("NWS forecast failed: {e}"))?;
    parse_grid(&body)
}

fn spans(layer: &Value) -> Vec<Span> {
    layer["values"]
        .as_array()
        .map(|vals| {
            vals.iter()
                .filter_map(|e| {
                    let value = e["value"].as_f64()?;
                    let (start, dur) = parse_valid_time(e["validTime"].as_str()?)?;
                    value.is_finite().then_some(Span { start, dur, value })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_grid(body: &str) -> Result<Grid, String> {
    let v = json(body, "NWS forecast")?;
    let p = &v["properties"];
    if !p.is_object() {
        return Err("NWS forecast: no data in response".to_string());
    }
    let snow_layer = &p["snowfallAmount"];
    let unit = snow_layer["uom"].as_str().unwrap_or("wmoUnit:mm");
    // Normalise whatever length unit the grid uses to centimetres.
    let to_cm = if unit.ends_with(":mm") {
        0.1
    } else if unit.ends_with(":cm") {
        1.0
    } else if unit.ends_with(":m") {
        100.0
    } else if unit.ends_with("in]") || unit.ends_with(":in") {
        2.54
    } else {
        0.1
    };
    let mut snow_cm = spans(snow_layer);
    for s in &mut snow_cm {
        s.value = (s.value * to_cm).max(0.0);
    }
    let temp_layer = &p["temperature"];
    let mut temp_c = spans(temp_layer);
    if temp_layer["uom"].as_str().is_some_and(|u| u.contains("degF")) {
        for s in &mut temp_c {
            s.value = (s.value - 32.0) / 1.8;
        }
    }
    Ok(Grid { snow_cm, temp_c })
}

/// Per-day forecast for `n_days` local days starting at `today`.
/// A day the grid does not cover has `snow_cm == None`.
pub fn forecast_days<Z: TimeZone>(grid: &Grid, tz: &Z, today: NaiveDate, n_days: u32) -> Vec<DaySnow> {
    let mut snow: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for s in &grid.snow_cm {
        for (date, part) in split_by_local_day(s.start, s.dur, s.value, tz) {
            *snow.entry(date).or_insert(0.0) += part;
        }
    }
    let mut temps: BTreeMap<NaiveDate, (f64, f64)> = BTreeMap::new();
    for s in &grid.temp_c {
        // A temperature holds for its whole interval; sample each day it touches.
        for (date, _) in split_by_local_day(s.start, s.dur, 1.0, tz) {
            let e = temps.entry(date).or_insert((s.value, s.value));
            e.0 = e.0.max(s.value);
            e.1 = e.1.min(s.value);
        }
    }
    (0..n_days as i64)
        .map(|i| {
            let date = today + Duration::days(i);
            DaySnow {
                date: date.to_string(),
                snow_cm: snow.get(&date).copied(),
                depth_cm: None,
                temp_hi_c: temps.get(&date).map(|t| t.0),
                temp_lo_c: temps.get(&date).map(|t| t.1),
                is_forecast: true,
                long_range: false,
            }
        })
        .collect()
}

/// A total is only reported when the grid covers at least this share of the
/// window; many offices forecast snowfall only three or four days out.
pub const MIN_WINDOW_COVERAGE: f64 = 0.7;

/// Forecast snow in the `hours` after `from`. `None` when the grid's
/// snowfall data does not cover enough of that window to give a fair total.
pub fn window_snow_cm(grid: &Grid, from: DateTime<Utc>, hours: i64) -> Option<f64> {
    let to = from + Duration::hours(hours);
    let mut covered_s = 0.0;
    let mut sum = 0.0;
    for s in &grid.snow_cm {
        let f = overlap_fraction(s.start, s.dur, from, to);
        if f > 0.0 {
            covered_s += f * s.dur.num_seconds() as f64;
            sum += s.value * f;
        }
    }
    (covered_s >= MIN_WINDOW_COVERAGE * (hours * 3600) as f64).then_some(sum)
}

const WINTER_WORDS: [&str; 9] =
    ["winter", "snow", "blizzard", "avalanche", "ice", "freez", "frost", "wind chill", "cold"];

pub fn fetch_alerts(http: &dyn Http, lat: f64, lon: f64) -> Result<Vec<Alert>, String> {
    let url = format!("{BASE}/alerts/active?point={lat:.4},{lon:.4}");
    let body = http.get(&url).map_err(|e| format!("NWS alerts failed: {e}"))?;
    parse_alerts(&body)
}

pub fn parse_alerts(body: &str) -> Result<Vec<Alert>, String> {
    let v = json(body, "NWS alerts")?;
    let feats = v["features"].as_array().ok_or("NWS alerts: no data in response")?;
    Ok(feats
        .iter()
        .filter_map(|f| {
            let p = &f["properties"];
            let event = p["event"].as_str()?.to_string();
            let lower = event.to_lowercase();
            if !WINTER_WORDS.iter().any(|w| lower.contains(w)) {
                return None;
            }
            Some(Alert {
                headline: p["headline"].as_str().unwrap_or(&event).to_string(),
                severity: p["severity"].as_str().unwrap_or("Unknown").to_string(),
                ends: p["ends"].as_str().or_else(|| p["expires"].as_str()).map(str::to_string),
                event,
            })
        })
        .collect())
}

pub fn fetch_outlook(http: &dyn Http, office: &str) -> Result<Option<String>, String> {
    let list = http
        .get(&format!("{BASE}/products/types/AFD/locations/{office}"))
        .map_err(|e| format!("NWS forecast discussion failed: {e}"))?;
    let v = json(&list, "NWS forecast discussion")?;
    let Some(first) = v["@graph"].as_array().and_then(|g| g.first()) else {
        return Ok(None);
    };
    let url = match (first["@id"].as_str(), first["id"].as_str()) {
        (Some(u), _) if u.starts_with("http") => u.to_string(),
        (_, Some(id)) => format!("{BASE}/products/{id}"),
        _ => return Ok(None),
    };
    let body = http.get(&url).map_err(|e| format!("NWS forecast discussion failed: {e}"))?;
    let v = json(&body, "NWS forecast discussion")?;
    Ok(v["productText"].as_str().and_then(extract_outlook))
}

/// Pull the readable forecast sections out of an Area Forecast Discussion.
/// Sections start with `.NAME...` and end with `&&`.
pub fn extract_outlook(text: &str) -> Option<String> {
    const WANT: [&str; 6] = ["KEY MESSAGES", "SYNOPSIS", "SHORT TERM", "LONG TERM", "DISCUSSION", "UPDATE"];
    let mut out = String::new();
    for block in text.split("&&") {
        let block = block.trim();
        let Some(pos) = block.find("\n.").map(|p| p + 1).or_else(|| block.starts_with('.').then_some(0)) else {
            continue;
        };
        let section = &block[pos..];
        let Some((head, body)) = section[1..].split_once("...") else { continue };
        let head_upper = head.to_uppercase();
        if head.len() > 60 || !WANT.iter().any(|w| head_upper.starts_with(w)) {
            continue;
        }
        // Re-flow hard-wrapped lines into paragraphs.
        let paras: Vec<String> = body
            .split("\n\n")
            .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|p| !p.is_empty())
            .collect();
        if paras.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&title_case(head.trim()));
        out.push('\n');
        out.push_str(&paras.join("\n\n"));
        if out.chars().count() > 1500 {
            break;
        }
    }
    if out.is_empty() {
        return None;
    }
    if out.chars().count() > 1500 {
        let cut: String = out.chars().take(1500).collect();
        let cut = cut.rsplit_once(' ').map(|(a, _)| a.to_string()).unwrap_or(cut);
        out = format!("{cut}…");
    }
    Some(out)
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::http::fake::FakeHttp;
    use serde_json::json;

    pub fn point_json() -> String {
        json!({"properties": {
            "gridId": "GJT", "cwa": "GJT", "gridX": 173, "gridY": 120,
            "forecastGridData": "https://api.weather.gov/gridpoints/GJT/173,120",
            "radarStation": "KGJX", "timeZone": "America/Denver"
        }})
        .to_string()
    }

    /// Snow: 6 mm on the evening of the 14th (local), 30 mm spanning the
    /// 15th, a null, and 12 mm on the 17th.
    pub fn grid_json() -> String {
        json!({"properties": {
            "snowfallAmount": {"uom": "wmoUnit:mm", "values": [
                {"validTime": "2026-11-14T18:00:00+00:00/PT6H", "value": 0.0},
                {"validTime": "2026-11-15T00:00:00+00:00/PT6H", "value": 6.0},
                {"validTime": "2026-11-15T06:00:00+00:00/PT6H", "value": 12.0},
                {"validTime": "2026-11-15T12:00:00+00:00/PT6H", "value": 30.0},
                {"validTime": "2026-11-15T18:00:00+00:00/PT6H", "value": null},
                {"validTime": "2026-11-16T00:00:00+00:00/P1D", "value": 0.0},
                {"validTime": "2026-11-17T12:00:00+00:00/PT6H", "value": 12.0},
                {"validTime": "garbage", "value": 99.0}
            ]},
            "temperature": {"uom": "wmoUnit:degC", "values": [
                {"validTime": "2026-11-14T19:00:00+00:00/PT1H", "value": -2.0},
                {"validTime": "2026-11-14T20:00:00+00:00/PT2H", "value": 1.5},
                {"validTime": "2026-11-15T12:00:00+00:00/PT1H", "value": -11.0},
                {"validTime": "2026-11-15T20:00:00+00:00/PT1H", "value": -4.0}
            ]}
        }})
        .to_string()
    }

    pub fn alerts_json() -> String {
        json!({"features": [
            {"properties": {"event": "Winter Storm Warning", "headline": "Winter Storm Warning until Monday 6 AM", "severity": "Severe", "ends": "2026-11-16T06:00:00-07:00"}},
            {"properties": {"event": "Red Flag Warning", "headline": "Fire weather", "severity": "Severe"}},
            {"properties": {"event": "Avalanche Warning", "severity": "Extreme", "expires": "2026-11-15T12:00:00-07:00"}}
        ]})
        .to_string()
    }

    pub const AFD: &str = "\n000\nFXUS65 KGJT 141000\nAFDGJT\n\nArea Forecast Discussion\nNational Weather Service Grand Junction CO\n\n.KEY MESSAGES...\n\n- Heavy mountain snow Sunday\n  into Monday.\n\n- Cold behind the front.\n\n&&\n\n.SHORT TERM /THROUGH SUNDAY NIGHT/...\nIssued at 300 AM MST\n\nA strong trough arrives Sunday\nwith snow levels near 7000 feet.\n\n&&\n\n.AVIATION...\nVFR until 18Z.\n\n&&\n\n.GJT WATCHES/WARNINGS/ADVISORIES...\nCO...Winter Storm Warning.\n\n&&\n$$\n";

    fn denver() -> chrono_tz::Tz {
        chrono_tz::America::Denver
    }

    #[test]
    fn point_parses() {
        let p = parse_point(&point_json()).unwrap();
        assert_eq!(p.office, "GJT");
        assert_eq!(p.grid_url, "https://api.weather.gov/gridpoints/GJT/173,120");
        assert_eq!(p.radar_station.as_deref(), Some("KGJX"));
        assert_eq!(p.tz, "America/Denver");
    }

    #[test]
    fn point_outside_coverage_is_error() {
        let body = json!({"status": 404, "title": "Data Unavailable For Requested Point"}).to_string();
        assert!(parse_point(&body).is_err());
        assert!(parse_point("<html>").is_err());
    }

    #[test]
    fn point_url_has_four_decimals() {
        let h = FakeHttp::new().ok("/points/39.6403,-106.3742", point_json());
        assert!(fetch_point(&h, 39.640312345, -106.374198765).is_ok());
    }

    #[test]
    fn grid_converts_mm_to_cm_and_skips_nulls_and_garbage() {
        let g = parse_grid(&grid_json()).unwrap();
        assert_eq!(g.snow_cm.len(), 6);
        assert!((g.snow_cm[1].value - 0.6).abs() < 1e-9);
        assert_eq!(g.temp_c.len(), 4);
    }

    #[test]
    fn grid_malformed_is_error() {
        assert!(parse_grid("not json").is_err());
        assert!(parse_grid("{}").is_err());
    }

    #[test]
    fn forecast_days_bucket_by_local_day() {
        let g = parse_grid(&grid_json()).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let days = forecast_days(&g, &denver(), today, 7);
        assert_eq!(days.len(), 7);
        assert!(days.iter().all(|d| d.is_forecast));
        // Nov 14 local (UTC-7): 18Z–00Z = 0, 00Z–06Z = 0.6 cm, plus 1 h of the 06Z–12Z block (1.2 cm / 6).
        assert!((days[0].snow_cm.unwrap() - 0.8).abs() < 1e-9, "{:?}", days[0]);
        // Nov 15 local: 5 h of 1.2 cm block + 3.0 cm + 17 h of a zero day.
        assert!((days[1].snow_cm.unwrap() - 4.0).abs() < 1e-9, "{:?}", days[1]);
        assert_eq!(days[2].snow_cm, Some(0.0));
        assert!((days[3].snow_cm.unwrap() - 1.2).abs() < 1e-9);
        // Beyond the grid: unknown, not zero.
        assert_eq!(days[5].snow_cm, None);
        assert_eq!(days[6].snow_cm, None);
        assert_eq!(days[0].temp_hi_c, Some(1.5));
        assert_eq!(days[0].temp_lo_c, Some(-2.0));
        assert_eq!(days[1].temp_lo_c, Some(-11.0));
        assert_eq!(days[4].temp_hi_c, None);
    }

    #[test]
    fn window_sums_partial_intervals() {
        let g = parse_grid(&grid_json()).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-11-14T19:00:00Z").unwrap().with_timezone(&Utc);
        // Next 24 h ends 19Z on the 15th: 0.6 + 1.2 + 3.0 = 4.8 cm.
        assert!((window_snow_cm(&g, now, 24).unwrap() - 4.8).abs() < 1e-9);
        assert!((window_snow_cm(&g, now, 72).unwrap() - 6.0).abs() < 1e-9);
        // The fixture's snowfall data ends after about three days: a 7-day total would mislead.
        assert_eq!(window_snow_cm(&g, now, 168), None);
        assert_eq!(window_snow_cm(&Grid::default(), now, 24), None);
    }

    #[test]
    fn alerts_keep_only_winter_events() {
        let a = parse_alerts(&alerts_json()).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].event, "Winter Storm Warning");
        assert_eq!(a[0].ends.as_deref(), Some("2026-11-16T06:00:00-07:00"));
        assert_eq!(a[1].headline, "Avalanche Warning");
        assert_eq!(a[1].ends.as_deref(), Some("2026-11-15T12:00:00-07:00"));
        assert!(parse_alerts("{}").is_err());
    }

    #[test]
    fn outlook_extracts_forecast_sections_only() {
        let o = extract_outlook(AFD).unwrap();
        assert!(o.starts_with("Key Messages\n- Heavy mountain snow Sunday into Monday."), "{o}");
        assert!(o.contains("Short Term /through Sunday Night/\nIssued at 300 AM MST\n\nA strong trough arrives Sunday with snow levels near 7000 feet."), "{o}");
        assert!(!o.contains("VFR"));
        assert!(!o.contains("WATCHES"));
        assert_eq!(extract_outlook("nothing here"), None);
    }

    #[test]
    fn outlook_is_capped() {
        let long = format!(".DISCUSSION...\n{}\n&&", "snow word ".repeat(500));
        let o = extract_outlook(&long).unwrap();
        assert!(o.chars().count() <= 1501);
        assert!(o.ends_with('…'));
    }

    #[test]
    fn outlook_fetch_follows_latest_product() {
        let h = FakeHttp::new()
            .ok("/products/types/AFD/locations/GJT", json!({"@graph": [{"@id": "https://api.weather.gov/products/abc", "id": "abc"}]}).to_string())
            .ok("/products/abc", json!({"productText": AFD}).to_string());
        assert!(fetch_outlook(&h, "GJT").unwrap().unwrap().contains("Heavy mountain snow"));
        let empty = FakeHttp::new().ok("AFD", json!({"@graph": []}).to_string());
        assert_eq!(fetch_outlook(&empty, "GJT").unwrap(), None);
    }
}

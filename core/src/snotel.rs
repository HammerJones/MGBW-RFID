//! Observed snow from the nearest SNOTEL station (USDA NRCS AWDB REST API).

use crate::geo::haversine_km;
use crate::http::Http;
use crate::model::{DaySnow, StationInfo};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const BASE: &str = "https://wcc.sc.egov.usda.gov/awdbRestApi/services/v1";
pub const MAX_STATION_KM: f64 = 60.0;
/// A one-day rise larger than this is treated as a sensor fault.
const SPIKE_CM: f64 = 100.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub triplet: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub elev_m: Option<f64>,
}

pub fn fetch_stations(http: &dyn Http) -> Result<Vec<Station>, String> {
    let url = format!("{BASE}/stations?stationTriplets=*:*:SNTL&activeOnly=true");
    let body = http.get(&url).map_err(|e| format!("SNOTEL station list failed: {e}"))?;
    parse_stations(&body)
}

pub fn parse_stations(body: &str) -> Result<Vec<Station>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("SNOTEL station list: unreadable response ({e})"))?;
    let arr = v.as_array().ok_or("SNOTEL station list: no data in response")?;
    Ok(arr
        .iter()
        .filter_map(|s| {
            Some(Station {
                triplet: s["stationTriplet"].as_str()?.to_string(),
                name: s["name"].as_str().unwrap_or("SNOTEL station").to_string(),
                lat: s["latitude"].as_f64()?,
                lon: s["longitude"].as_f64()?,
                elev_m: s["elevation"].as_f64().map(|ft| ft * 0.3048),
            })
        })
        .collect())
}

pub fn nearest_station(stations: &[Station], lat: f64, lon: f64) -> Option<StationInfo> {
    stations
        .iter()
        .map(|s| (haversine_km(lat, lon, s.lat, s.lon), s))
        .filter(|(d, _)| d.is_finite() && *d <= MAX_STATION_KM)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(d, s)| StationInfo {
            name: s.name.clone(),
            triplet: s.triplet.clone(),
            distance_km: d,
            elev_m: s.elev_m,
        })
}

/// `days` days of history ending yesterday. Each day's depth is the reading
/// at the end of that day; new snow is the rise since the previous reading.
pub fn fetch_history(http: &dyn Http, station: &StationInfo, today: NaiveDate, days: u32) -> Result<Vec<DaySnow>, String> {
    let begin = today - Duration::days(days as i64 + 1);
    let url = format!(
        "{BASE}/data?stationTriplets={}&elements=SNWD&duration=DAILY&periodRef=END&beginDate={}&endDate={}",
        station.triplet, begin, today
    );
    let body = http.get(&url).map_err(|e| format!("SNOTEL data failed: {e}"))?;
    let depths = parse_depths_cm(&body)?;
    Ok(history_from_depths(&depths, today, days))
}

/// Date → snow depth in cm. Missing or negative readings are left out.
pub fn parse_depths_cm(body: &str) -> Result<BTreeMap<NaiveDate, f64>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("SNOTEL data: unreadable response ({e})"))?;
    let arr = v.as_array().ok_or("SNOTEL data: no data in response")?;
    let mut out = BTreeMap::new();
    for station in arr {
        for series in station["data"].as_array().into_iter().flatten() {
            let el = &series["stationElement"];
            if el["elementCode"].as_str() != Some("SNWD") {
                continue;
            }
            let to_cm = match el["storedUnitCode"].as_str() {
                Some("cm") => 1.0,
                Some("mm") => 0.1,
                Some("m") => 100.0,
                _ => 2.54,
            };
            for val in series["values"].as_array().into_iter().flatten() {
                let date = val["date"].as_str().and_then(|d| d.get(..10)).and_then(|d| d.parse::<NaiveDate>().ok());
                if let (Some(date), Some(x)) = (date, val["value"].as_f64()) {
                    if x.is_finite() && x >= 0.0 {
                        out.insert(date, x * to_cm);
                    }
                }
            }
        }
    }
    Ok(out)
}

/// A reading that jumps away from both neighbouring days by more than this,
/// while those neighbours agree with each other, is a sensor glitch.
const OUTLIER_CM: f64 = 30.0;

fn without_outliers(depths: &BTreeMap<NaiveDate, f64>) -> BTreeMap<NaiveDate, f64> {
    depths
        .iter()
        .filter(|(date, d)| {
            let prev = depths.get(&(**date - Duration::days(1)));
            let next = depths.get(&(**date + Duration::days(1)));
            match (prev, next) {
                (Some(p), Some(n)) => {
                    let same_side = (**d - p) * (**d - n) > 0.0;
                    !(same_side && (**d - p).abs() > OUTLIER_CM && (**d - n).abs() > OUTLIER_CM && (p - n).abs() <= OUTLIER_CM)
                }
                _ => true,
            }
        })
        .map(|(k, v)| (*k, *v))
        .collect()
}

pub fn history_from_depths(depths: &BTreeMap<NaiveDate, f64>, today: NaiveDate, days: u32) -> Vec<DaySnow> {
    let depths = without_outliers(depths);
    (1..=days as i64)
        .rev()
        .map(|back| {
            let date = today - Duration::days(back);
            let depth = depths.get(&date).copied();
            let prev = depths.get(&(date - Duration::days(1))).copied();
            // New snow needs two consecutive good readings; anything else is unknown.
            let (snow, depth) = match (depth, prev) {
                // The newest reading has no later neighbour to check it against.
                (Some(d), Some(p)) if d - p > SPIKE_CM => (None, None),
                (Some(d), Some(p)) => (Some((d - p).max(0.0)), Some(d)),
                (d, _) => (None, d),
            };
            DaySnow {
                date: date.to_string(),
                snow_cm: snow,
                depth_cm: depth,
                temp_hi_c: None,
                temp_lo_c: None,
                is_forecast: false,
                long_range: false,
            }
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    pub fn stations_json() -> String {
        json!([
            {"stationTriplet": "842:CO:SNTL", "name": "Vail Mountain", "elevation": 10290.0, "latitude": 39.61765, "longitude": -106.38019},
            {"stationTriplet": "415:CO:SNTL", "name": "Copper Mountain", "elevation": 10500.0, "latitude": 39.48917, "longitude": -106.17154},
            {"stationTriplet": "bad", "name": "No coordinates"}
        ])
        .to_string()
    }

    pub fn data_json(values: &[(&str, Option<f64>)]) -> String {
        let vals: Vec<Value> = values.iter().map(|(d, v)| json!({"date": d, "value": v})).collect();
        json!([{"stationTriplet": "842:CO:SNTL", "data": [
            {"stationElement": {"elementCode": "WTEQ", "storedUnitCode": "in"}, "values": [{"date": "2026-11-13", "value": 99.0}]},
            {"stationElement": {"elementCode": "SNWD", "storedUnitCode": "in"}, "values": vals}
        ]}])
        .to_string()
    }

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn stations_parse_and_skip_incomplete() {
        let s = parse_stations(&stations_json()).unwrap();
        assert_eq!(s.len(), 2);
        assert!((s[0].elev_m.unwrap() - 3136.4).abs() < 0.1);
        assert!(parse_stations("{}").is_err());
    }

    #[test]
    fn nearest_picks_closest_within_range() {
        let s = parse_stations(&stations_json()).unwrap();
        let vail = nearest_station(&s, 39.6403, -106.3742).unwrap();
        assert_eq!(vail.triplet, "842:CO:SNTL");
        assert!(vail.distance_km < 5.0);
        let copper = nearest_station(&s, 39.5022, -106.1497).unwrap();
        assert_eq!(copper.name, "Copper Mountain");
    }

    #[test]
    fn nearest_none_when_too_far() {
        let s = parse_stations(&stations_json()).unwrap();
        // Killington, Vermont: no SNOTEL anywhere near.
        assert_eq!(nearest_station(&s, 43.6045, -72.8201), None);
        assert_eq!(nearest_station(&[], 39.6, -106.3), None);
    }

    #[test]
    fn depths_convert_inches_and_ignore_other_elements() {
        let m = parse_depths_cm(&data_json(&[("2026-11-12", Some(10.0)), ("2026-11-13 00:00", Some(12.0)), ("2026-11-14", None), ("2026-11-15", Some(-1.0))])).unwrap();
        assert_eq!(m.len(), 2);
        assert!((m[&d("2026-11-12")] - 25.4).abs() < 1e-9);
        assert!((m[&d("2026-11-13")] - 30.48).abs() < 1e-9);
        assert!(parse_depths_cm("oops").is_err());
    }

    #[test]
    fn new_snow_is_positive_delta_with_gaps_and_settling() {
        let mut m = BTreeMap::new();
        for (date, cm) in [("2026-11-09", 100.0), ("2026-11-10", 110.0), ("2026-11-11", 105.0), ("2026-11-13", 120.0)] {
            m.insert(d(date), cm);
        }
        let h = history_from_depths(&m, d("2026-11-14"), 5);
        let snow: Vec<_> = h.iter().map(|x| x.snow_cm).collect();
        assert_eq!(snow, vec![None, Some(10.0), Some(0.0), None, None]);
        let depth: Vec<_> = h.iter().map(|x| x.depth_cm).collect();
        assert_eq!(depth, vec![Some(100.0), Some(110.0), Some(105.0), None, Some(120.0)]);
        assert_eq!(h[0].date, "2026-11-09");
        assert!(h.iter().all(|x| !x.is_forecast));
    }

    #[test]
    fn one_day_spike_is_dropped_and_not_counted_as_snow() {
        let mut m = BTreeMap::new();
        for (date, cm) in [("2026-11-11", 50.0), ("2026-11-12", 200.0), ("2026-11-13", 52.0)] {
            m.insert(d(date), cm);
        }
        let h = history_from_depths(&m, d("2026-11-14"), 2);
        assert_eq!((h[0].snow_cm, h[0].depth_cm), (None, None));
        // The day after has no trustworthy reading before it: unknown, not zero.
        assert_eq!((h[1].snow_cm, h[1].depth_cm), (None, Some(52.0)));
    }

    #[test]
    fn one_day_dip_does_not_become_phantom_snow() {
        let mut m = BTreeMap::new();
        for (date, cm) in [("2026-11-10", 100.0), ("2026-11-11", 100.0), ("2026-11-12", 40.0), ("2026-11-13", 100.0)] {
            m.insert(d(date), cm);
        }
        let h = history_from_depths(&m, d("2026-11-14"), 3);
        let snow: Vec<_> = h.iter().map(|x| x.snow_cm).collect();
        assert_eq!(snow, vec![Some(0.0), None, None]);
        assert_eq!(crate::history_totals(&h), (None, Some(0.0), Some(0.0), Some(100.0)));
    }

    #[test]
    fn a_real_storm_is_not_mistaken_for_a_glitch() {
        let mut m = BTreeMap::new();
        for (date, cm) in [("2026-11-10", 50.0), ("2026-11-11", 95.0), ("2026-11-12", 130.0), ("2026-11-13", 120.0)] {
            m.insert(d(date), cm);
        }
        let h = history_from_depths(&m, d("2026-11-14"), 3);
        let snow: Vec<_> = h.iter().map(|x| x.snow_cm).collect();
        assert_eq!(snow, vec![Some(45.0), Some(35.0), Some(0.0)]);
    }

    #[test]
    fn newest_reading_spike_is_dropped() {
        let mut m = BTreeMap::new();
        m.insert(d("2026-11-12"), 50.0);
        m.insert(d("2026-11-13"), 200.0);
        let h = history_from_depths(&m, d("2026-11-14"), 1);
        assert_eq!((h[0].snow_cm, h[0].depth_cm), (None, None));
    }
}

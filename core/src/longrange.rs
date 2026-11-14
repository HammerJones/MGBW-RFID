//! Days beyond the local NWS forecast, from NOAA's GFS global model as
//! served by Open-Meteo (free, no key). Good for spotting a pattern a week
//! or two out; not for trusting amounts.

use crate::http::Http;
use chrono::NaiveDate;
use serde_json::Value;

pub const LONG_RANGE_DAYS: u32 = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelDay {
    pub date: NaiveDate,
    pub snow_cm: Option<f64>,
    pub temp_hi_c: Option<f64>,
    pub temp_lo_c: Option<f64>,
}

pub fn url(lat: f64, lon: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/gfs?latitude={lat:.4}&longitude={lon:.4}&daily=snowfall_sum,temperature_2m_max,temperature_2m_min&forecast_days={LONG_RANGE_DAYS}&timezone=auto"
    )
}

pub fn fetch(http: &dyn Http, lat: f64, lon: f64) -> Result<Vec<ModelDay>, String> {
    let body = http.get(&url(lat, lon)).map_err(|e| format!("Long-range forecast failed: {e}"))?;
    parse(&body)
}

pub fn parse(body: &str) -> Result<Vec<ModelDay>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("Long-range forecast: unreadable response ({e})"))?;
    let daily = &v["daily"];
    let times = daily["time"].as_array().ok_or("Long-range forecast: no data in response")?;
    // Open-Meteo reports snowfall in cm and temperature in °C unless asked otherwise; check anyway.
    let units = &v["daily_units"];
    let snow_scale = match units["snowfall_sum"].as_str() {
        Some("inch") => 2.54,
        Some("mm") => 0.1,
        _ => 1.0,
    };
    let fahrenheit = units["temperature_2m_max"].as_str().is_some_and(|u| u.contains('F'));
    let temp = |x: f64| if fahrenheit { (x - 32.0) / 1.8 } else { x };
    let at = |key: &str, i: usize| daily[key].as_array().and_then(|a| a.get(i)).and_then(Value::as_f64).filter(|x| x.is_finite());
    Ok(times
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            Some(ModelDay {
                date: t.as_str()?.parse().ok()?,
                snow_cm: at("snowfall_sum", i).map(|x| (x * snow_scale).max(0.0)),
                temp_hi_c: at("temperature_2m_max", i).map(temp),
                temp_lo_c: at("temperature_2m_min", i).map(temp),
            })
        })
        .collect())
}

/// Fill gaps in the NWS days from the model and append the days beyond them.
/// NWS numbers always win where they exist.
pub fn merge(mut nws: Vec<crate::model::DaySnow>, model: &[ModelDay], today: NaiveDate) -> Vec<crate::model::DaySnow> {
    for day in &mut nws {
        let Some(m) = model.iter().find(|m| m.date.to_string() == day.date) else { continue };
        if day.snow_cm.is_none() && m.snow_cm.is_some() {
            day.snow_cm = m.snow_cm;
            day.long_range = true;
        }
        day.temp_hi_c = day.temp_hi_c.or(m.temp_hi_c);
        day.temp_lo_c = day.temp_lo_c.or(m.temp_lo_c);
    }
    let last = nws.last().and_then(|d| d.date.parse::<NaiveDate>().ok());
    for m in model {
        if m.date < today || last.is_some_and(|l| m.date <= l) {
            continue;
        }
        nws.push(crate::model::DaySnow {
            date: m.date.to_string(),
            snow_cm: m.snow_cm,
            depth_cm: None,
            temp_hi_c: m.temp_hi_c,
            temp_lo_c: m.temp_lo_c,
            is_forecast: true,
            long_range: true,
        });
    }
    nws
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::model::DaySnow;
    use serde_json::json;

    /// Sixteen days from 14 Nov 2026; snow on the 15th, 19th, 20th and 25th.
    pub fn gfs_json() -> String {
        let dates: Vec<String> = (14..30).map(|d| format!("2026-11-{d}")).collect();
        let mut snow = vec![json!(0.0); 16];
        snow[1] = json!(9.9);
        snow[5] = json!(2.0);
        snow[6] = json!(7.0);
        snow[11] = json!(12.5);
        snow[15] = json!(null);
        let hi: Vec<f64> = (0..16).map(|i| 3.0 - i as f64 * 0.5).collect();
        let lo: Vec<f64> = (0..16).map(|i| -5.0 - i as f64 * 0.5).collect();
        json!({"timezone": "America/Denver",
               "daily_units": {"time": "iso8601", "snowfall_sum": "cm", "temperature_2m_max": "°C"},
               "daily": {"time": dates, "snowfall_sum": snow, "temperature_2m_max": hi, "temperature_2m_min": lo}})
        .to_string()
    }

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn nws_day(date: &str, snow: Option<f64>, hi: Option<f64>) -> DaySnow {
        DaySnow { date: date.into(), snow_cm: snow, depth_cm: None, temp_hi_c: hi, temp_lo_c: None, is_forecast: true, long_range: false }
    }

    #[test]
    fn parses_days_and_nulls() {
        let days = parse(&gfs_json()).unwrap();
        assert_eq!(days.len(), 16);
        assert_eq!(days[1], ModelDay { date: d("2026-11-15"), snow_cm: Some(9.9), temp_hi_c: Some(2.5), temp_lo_c: Some(-5.5) });
        assert_eq!(days[15].snow_cm, None);
        assert!(parse("{}").is_err());
        assert!(parse("nope").is_err());
    }

    #[test]
    fn converts_other_units() {
        let body = json!({"daily_units": {"snowfall_sum": "inch", "temperature_2m_max": "°F"},
            "daily": {"time": ["2026-11-14"], "snowfall_sum": [2.0], "temperature_2m_max": [32.0], "temperature_2m_min": [14.0]}}).to_string();
        let day = &parse(&body).unwrap()[0];
        assert!((day.snow_cm.unwrap() - 5.08).abs() < 1e-9);
        assert!((day.temp_hi_c.unwrap()).abs() < 1e-9);
        assert!((day.temp_lo_c.unwrap() + 10.0).abs() < 1e-9);
    }

    #[test]
    fn nws_wins_and_model_fills_gaps_then_extends() {
        let model = parse(&gfs_json()).unwrap();
        let nws = vec![nws_day("2026-11-14", Some(1.0), Some(8.0)), nws_day("2026-11-15", Some(4.0), None), nws_day("2026-11-16", None, None)];
        let out = merge(nws, &model, d("2026-11-14"));
        assert_eq!(out.len(), 16);
        // NWS value kept, even though the model says 9.9.
        assert_eq!((out[1].snow_cm, out[1].long_range), (Some(4.0), false));
        assert_eq!(out[1].temp_hi_c, Some(2.5)); // missing temperature borrowed
        assert_eq!(out[0].temp_hi_c, Some(8.0));
        // Gap filled and flagged.
        assert_eq!((out[2].snow_cm, out[2].long_range), (Some(0.0), true));
        // Extension.
        assert_eq!(out[3].date, "2026-11-17");
        assert!(out[3..].iter().all(|x| x.long_range && x.is_forecast));
        assert_eq!(out[11].snow_cm, Some(12.5));
        assert_eq!(out[15].snow_cm, None);
    }

    #[test]
    fn without_nws_the_model_starts_today_not_before() {
        let model = parse(&gfs_json()).unwrap();
        let out = merge(Vec::new(), &model, d("2026-11-16"));
        assert_eq!(out.len(), 14);
        assert_eq!(out[0].date, "2026-11-16");
        assert_eq!(merge(vec![nws_day("2026-11-14", None, None)], &[], d("2026-11-14")).len(), 1);
    }
}

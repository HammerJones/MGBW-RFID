//! Plain data shared between the Rust core and the Android UI.
//! Everything is metric here; the UI converts for display.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Resort {
    pub id: String,
    pub name: String,
    pub state: String,
    pub lat: f64,
    pub lon: f64,
    pub base_elev_m: Option<f64>,
    pub summit_elev_m: Option<f64>,
}

/// One calendar day (resort-local) of observed or forecast snow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct DaySnow {
    /// `YYYY-MM-DD`, local to the resort.
    pub date: String,
    pub snow_cm: Option<f64>,
    pub depth_cm: Option<f64>,
    pub temp_hi_c: Option<f64>,
    pub temp_lo_c: Option<f64>,
    pub is_forecast: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Alert {
    pub event: String,
    pub headline: String,
    pub severity: String,
    pub ends: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct NewsItem {
    pub title: String,
    pub source: String,
    pub url: String,
    /// Unix seconds; 0 when the feed gave no usable date.
    pub published_epoch_s: i64,
    /// Snowfall amount mentioned in the headline, if any.
    pub reported_snow_cm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct StationInfo {
    pub name: String,
    pub triplet: String,
    pub distance_km: f64,
    pub elev_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Snapshot {
    pub resort: Resort,
    /// History days (oldest first) followed by forecast days starting today.
    pub days: Vec<DaySnow>,
    pub last_24h_cm: Option<f64>,
    pub last_72h_cm: Option<f64>,
    pub last_7d_cm: Option<f64>,
    pub next_24h_cm: Option<f64>,
    pub next_72h_cm: Option<f64>,
    pub next_7d_cm: Option<f64>,
    pub depth_cm: Option<f64>,
    pub station: Option<StationInfo>,
    pub alerts: Vec<Alert>,
    pub news: Vec<NewsItem>,
    pub outlook: Option<String>,
    pub radar_station: Option<String>,
    pub fetched_at_epoch_s: i64,
    /// True when this is cached data returned because a refresh failed.
    pub stale: bool,
    /// Human-readable notes about sources that could not be loaded.
    pub problems: Vec<String>,
}

/// A resort on the home screen.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Card {
    pub resort: Resort,
    pub notify: bool,
    pub snapshot: Option<Snapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Settings {
    pub metric: bool,
    /// Forecast snow over the next 72 hours that triggers a notification.
    pub notify_threshold_cm: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { metric: false, notify_threshold_cm: 15.24 }
    }
}

/// Something the UI should post as a system notification.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SnowNotice {
    pub resort_id: String,
    pub resort_name: String,
    /// Notices with the same resort and tag replace each other: `storm`, or `alert:<event>`.
    pub tag: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, uniffi::Error)]
pub enum SnowError {
    #[error("No resort with that id")]
    NotFound,
    #[error("{msg}")]
    Invalid { msg: String },
    #[error("{msg}")]
    Network { msg: String },
}

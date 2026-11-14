//! SnowTrack core: snowfall tracking and forecasting for ski resorts.
//!
//! The Android app talks to this crate through one object, [`SnowTrack`].
//! Everything it returns is metric; the UI converts for display.

uniffi::setup_scaffolding!();

pub mod catalog;
pub mod geo;
pub mod http;
pub mod interval;
pub mod model;
pub mod news;
pub mod notify;
pub mod nws;
pub mod radar;
pub mod snotel;
pub mod store;

use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};
use http::{Http, UreqHttp};
use model::*;
use std::sync::{Arc, Mutex, MutexGuard};
use store::{State, StationCache, Store, Tracked};

pub const HISTORY_DAYS: u32 = 7;
pub const FORECAST_DAYS: u32 = 7;
pub const MAX_RESORTS: usize = 40;
const STATION_LIST_MAX_AGE_S: i64 = 30 * 24 * 3600;

type Clock = Box<dyn Fn() -> DateTime<Utc> + Send + Sync>;

#[derive(uniffi::Object)]
pub struct SnowTrack {
    store: Store,
    state: Mutex<State>,
    http: Arc<dyn Http>,
    clock: Clock,
}

impl SnowTrack {
    pub fn with_parts(data_dir: &str, http: Arc<dyn Http>, clock: Clock) -> Self {
        let store = Store::new(data_dir);
        let state = Mutex::new(store.load_state());
        SnowTrack { store, state, http, clock }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, state: &State) -> Result<(), SnowError> {
        self.store.save_state(state).map_err(|msg| SnowError::Invalid { msg })
    }

    /// Change one tracked resort and persist. Missing ids are ignored
    /// (the resort may have been removed while a refresh was running).
    fn update_tracked(&self, id: &str, f: impl FnOnce(&mut Tracked)) {
        let mut state = self.state();
        if let Some(t) = state.tracked.iter_mut().find(|t| t.resort.id == id) {
            f(t);
            let _ = self.store.save_state(&state);
        }
    }

    fn stations(&self, now: DateTime<Utc>) -> Result<Vec<snotel::Station>, String> {
        let cached = self.store.load_stations();
        if let Some(c) = &cached {
            if now.timestamp() - c.fetched_at_epoch_s < STATION_LIST_MAX_AGE_S && !c.stations.is_empty() {
                return Ok(c.stations.clone());
            }
        }
        match snotel::fetch_stations(self.http.as_ref()) {
            Ok(stations) if !stations.is_empty() => {
                let _ = self.store.save_stations(&StationCache { fetched_at_epoch_s: now.timestamp(), stations: stations.clone() });
                Ok(stations)
            }
            Ok(_) => Err("SNOTEL station list was empty".to_string()),
            // An old list is far better than none: stations rarely move.
            Err(e) => cached.map(|c| c.stations).filter(|s| !s.is_empty()).ok_or(e),
        }
    }

    fn do_refresh(&self, id: &str) -> Result<Snapshot, SnowError> {
        let tracked = self.state().tracked.iter().find(|t| t.resort.id == id).cloned().ok_or(SnowError::NotFound)?;
        let resort = tracked.resort.clone();
        let http = self.http.as_ref();
        let now = (self.clock)();
        let mut problems: Vec<String> = Vec::new();
        let mut successes = 0usize;

        // 1. Where is this on the NWS grid? (cached after the first time)
        let mut point = tracked.point.clone();
        if point.is_none() {
            match nws::fetch_point(http, resort.lat, resort.lon) {
                Ok(p) => {
                    successes += 1;
                    point = Some(p);
                }
                Err(e) => problems.push(e),
            }
        }

        // 2. Nearest SNOTEL station (cached, including "none nearby").
        let station: Option<StationInfo> = match tracked.station_checked {
            true => tracked.station.clone(),
            false => match self.stations(now) {
                Ok(list) => {
                    successes += 1;
                    let found = snotel::nearest_station(&list, resort.lat, resort.lon);
                    let save = found.clone();
                    self.update_tracked(id, |t| {
                        t.station_checked = true;
                        t.station = save;
                    });
                    found
                }
                Err(e) => {
                    problems.push(e);
                    None
                }
            },
        };

        let tz: Option<chrono_tz::Tz> = point.as_ref().and_then(|p| p.tz.parse().ok());
        let today = local_today(now, tz, resort.lon);

        // 3. Everything else in parallel.
        let (grid, alerts, outlook, history, news) = std::thread::scope(|s| {
            let grid = s.spawn(|| point.as_ref().map(|p| nws::fetch_grid(http, p)));
            let alerts = s.spawn(|| nws::fetch_alerts(http, resort.lat, resort.lon));
            let outlook = s.spawn(|| point.as_ref().map(|p| nws::fetch_outlook(http, &p.office)));
            let history = s.spawn(|| station.as_ref().map(|st| snotel::fetch_history(http, st, today, HISTORY_DAYS)));
            let news = s.spawn(|| news::fetch_news(http, &resort));
            let join = |what: &str| format!("{what} crashed while loading");
            (
                grid.join().unwrap_or_else(|_| Some(Err(join("Forecast")))),
                alerts.join().unwrap_or_else(|_| Err(join("Alerts"))),
                outlook.join().unwrap_or_else(|_| Some(Err(join("Outlook")))),
                history.join().unwrap_or_else(|_| Some(Err(join("Snow history")))),
                news.join().unwrap_or_else(|_| Err(join("News"))),
            )
        });

        // A cached grid address can go out of date when NWS re-maps an office.
        let mut grid = grid;
        if matches!(grid, Some(Err(_))) && tracked.point.is_some() {
            if let Ok(fresh) = nws::fetch_point(http, resort.lat, resort.lon) {
                if Some(&fresh) != point.as_ref() {
                    grid = Some(nws::fetch_grid(http, &fresh));
                    point = Some(fresh);
                }
            }
        }

        let mut tally = Tally { successes, problems };
        let grid: Option<nws::Grid> = grid.and_then(|r| tally.take(r));
        let alerts: Vec<Alert> = tally.take(alerts).unwrap_or_default();
        let outlook: Option<String> = outlook.and_then(|r| tally.take(r)).flatten();
        let history: Vec<DaySnow> = history.and_then(|r| tally.take(r)).unwrap_or_default();
        let news: Vec<NewsItem> = tally.take(news).unwrap_or_default();
        let Tally { successes, problems } = tally;

        if successes == 0 {
            // Nothing reachable: fall back to what we had.
            return match self.store.load_snapshot(id) {
                Some(mut cached) => {
                    cached.stale = true;
                    cached.problems = problems;
                    Ok(cached)
                }
                None => Err(SnowError::Network {
                    msg: problems.first().cloned().unwrap_or_else(|| "Could not reach any data source".to_string()),
                }),
            };
        }

        if point != tracked.point {
            let p = point.clone();
            self.update_tracked(id, |t| t.point = p);
        }

        let (mut forecast, mut next) = (Vec::new(), (None, None, None));
        if let Some(g) = &grid {
            forecast = match tz {
                Some(tz) => nws::forecast_days(g, &tz, today, FORECAST_DAYS),
                None => nws::forecast_days(g, &lon_offset(resort.lon), today, FORECAST_DAYS),
            };
            next = (nws::window_snow_cm(g, now, 24), nws::window_snow_cm(g, now, 72), nws::window_snow_cm(g, now, 168));
        }
        let (last_24h_cm, last_72h_cm, last_7d_cm, depth_cm) = history_totals(&history);

        let mut days = history;
        days.extend(forecast);
        let snap = Snapshot {
            resort,
            days,
            last_24h_cm,
            last_72h_cm,
            last_7d_cm,
            next_24h_cm: next.0,
            next_72h_cm: next.1,
            next_7d_cm: next.2,
            depth_cm,
            station,
            alerts,
            news,
            outlook,
            radar_station: point.and_then(|p| p.radar_station),
            fetched_at_epoch_s: now.timestamp(),
            stale: false,
            problems,
        };
        // Only keep it if the resort is still on the list.
        if self.state().tracked.iter().any(|t| t.resort.id == id) {
            let _ = self.store.save_snapshot(&snap);
        }
        Ok(snap)
    }
}

/// Counts sources that answered and collects messages from those that did not.
struct Tally {
    successes: usize,
    problems: Vec<String>,
}

impl Tally {
    fn take<T>(&mut self, r: Result<T, String>) -> Option<T> {
        match r {
            Ok(v) => {
                self.successes += 1;
                Some(v)
            }
            Err(e) => {
                self.problems.push(e);
                None
            }
        }
    }
}

fn lon_offset(lon: f64) -> FixedOffset {
    let hours = (lon / 15.0).round().clamp(-12.0, 12.0) as i32;
    FixedOffset::east_opt(hours * 3600).unwrap_or_else(|| FixedOffset::east_opt(0).expect("zero offset"))
}

fn local_today(now: DateTime<Utc>, tz: Option<chrono_tz::Tz>, lon: f64) -> NaiveDate {
    match tz {
        Some(tz) => now.with_timezone(&tz).date_naive(),
        None => now.with_timezone(&lon_offset(lon)).date_naive(),
    }
}

/// (yesterday, last 3 days, last 7 days, latest depth) from history days, oldest first.
pub fn history_totals(history: &[DaySnow]) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let sum_last = |n: usize| -> Option<f64> {
        let vals: Vec<f64> = history.iter().rev().take(n).filter_map(|d| d.snow_cm).collect();
        (!vals.is_empty()).then(|| vals.iter().sum())
    };
    let last_24h = history.last().and_then(|d| d.snow_cm);
    let depth = history.iter().rev().find_map(|d| d.depth_cm);
    (last_24h, sum_last(3), sum_last(7), depth)
}

#[uniffi::export]
impl SnowTrack {
    /// `data_dir` is the app's private files directory.
    #[uniffi::constructor]
    pub fn new(data_dir: String) -> Arc<Self> {
        Arc::new(Self::with_parts(&data_dir, Arc::new(UreqHttp::new()), Box::new(Utc::now)))
    }

    /// Resorts from the bundled catalog matching a name or state.
    pub fn search_catalog(&self, query: String) -> Vec<Resort> {
        let have: Vec<String> = self.state().tracked.iter().map(|t| t.resort.id.clone()).collect();
        catalog::search(&query, 200).into_iter().filter(|r| !have.contains(&r.id)).take(60).collect()
    }

    /// The home screen: every added resort with its last saved data.
    pub fn cards(&self) -> Vec<Card> {
        let tracked = self.state().tracked.clone();
        tracked
            .into_iter()
            .map(|t| Card { snapshot: self.store.load_snapshot(&t.resort.id), notify: t.notify, resort: t.resort })
            .collect()
    }

    pub fn add_resort(&self, resort: Resort) -> Result<(), SnowError> {
        if resort.id.trim().is_empty() || resort.name.trim().is_empty() {
            return Err(SnowError::Invalid { msg: "Resort needs a name".into() });
        }
        if !geo::in_us_bounds(resort.lat, resort.lon) {
            return Err(SnowError::Invalid { msg: "NOAA forecasts only cover the United States".into() });
        }
        let mut state = self.state();
        if state.tracked.iter().any(|t| t.resort.id == resort.id) {
            return Err(SnowError::Invalid { msg: format!("{} is already on your list", resort.name) });
        }
        if state.tracked.len() >= MAX_RESORTS {
            return Err(SnowError::Invalid { msg: format!("You can track up to {MAX_RESORTS} resorts") });
        }
        state.tracked.push(Tracked { resort, notify: false, notify_state: Default::default(), point: None, station_checked: false, station: None });
        self.save(&state)
    }

    /// Add a place that is not in the catalog.
    pub fn add_custom(&self, name: String, lat: f64, lon: f64) -> Result<Resort, SnowError> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() || name.chars().count() > 60 {
            return Err(SnowError::Invalid { msg: "Give it a name up to 60 characters".into() });
        }
        if !lat.is_finite() || !lon.is_finite() {
            return Err(SnowError::Invalid { msg: "Latitude and longitude must be numbers".into() });
        }
        let base = catalog::slug(&name, "custom");
        let mut id = base.clone();
        let mut n = 2;
        while self.state().tracked.iter().any(|t| t.resort.id == id) || catalog::all().iter().any(|r| r.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let resort = Resort { id, name, state: String::new(), lat, lon, base_elev_m: None, summit_elev_m: None };
        self.add_resort(resort.clone())?;
        Ok(resort)
    }

    pub fn remove_resort(&self, id: String) {
        let mut state = self.state();
        let before = state.tracked.len();
        state.tracked.retain(|t| t.resort.id != id);
        if state.tracked.len() != before {
            let _ = self.store.save_state(&state);
            self.store.delete_snapshot(&id);
        }
    }

    /// Switch the bell for one resort on or off.
    pub fn set_notify(&self, id: String, enabled: bool) {
        self.update_tracked(&id, |t| {
            t.notify = enabled;
            if !enabled {
                t.notify_state = Default::default();
            }
        });
    }

    /// Last saved data for a resort, without touching the network.
    pub fn snapshot(&self, id: String) -> Option<Snapshot> {
        self.store.load_snapshot(&id)
    }

    /// Fetch fresh data. Blocks on the network: call off the main thread.
    /// If nothing can be reached, returns the saved data with `stale` set.
    pub fn refresh(&self, id: String) -> Result<Snapshot, SnowError> {
        self.do_refresh(&id)
    }

    /// Refresh every resort with its bell on and return what is worth a
    /// notification. Each storm or alert is reported once.
    pub fn check_notifications(&self) -> Vec<SnowNotice> {
        let (ids, settings): (Vec<String>, Settings) = {
            let s = self.state();
            (s.tracked.iter().filter(|t| t.notify).map(|t| t.resort.id.clone()).collect(), s.settings.clone())
        };
        let mut out = Vec::new();
        for id in ids {
            let Ok(snap) = self.do_refresh(&id) else { continue };
            let Some(prev) = self.state().tracked.iter().find(|t| t.resort.id == id && t.notify).map(|t| t.notify_state.clone()) else {
                continue;
            };
            let (notices, next) = notify::decide(&prev, &snap, settings.notify_threshold_cm, settings.metric);
            if next != prev {
                self.update_tracked(&id, |t| t.notify_state = next);
            }
            out.extend(notices);
        }
        out
    }

    pub fn settings(&self) -> Settings {
        self.state().settings.clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        let mut state = self.state();
        let t = settings.notify_threshold_cm;
        state.settings = Settings {
            metric: settings.metric,
            notify_threshold_cm: if t.is_finite() { t.clamp(2.5, 150.0) } else { Settings::default().notify_threshold_cm },
        };
        let _ = self.store.save_state(&state);
    }

    /// Raster tile URL (WMS; the app fills in `{bbox-epsg-3857}`) for the radar layer covering this location;
    /// `time` is one of `radar_times`, or `None` for the latest frame.
    pub fn radar_tile_template(&self, lat: f64, lon: f64, time: Option<String>) -> String {
        radar::tile_template(radar::region(lat, lon), time.as_deref())
    }

    /// Recent radar frame times for this location, oldest first. Empty if
    /// unavailable. Blocks on the network.
    pub fn radar_times(&self, lat: f64, lon: f64) -> Vec<String> {
        radar::fetch_times(self.http.as_ref(), radar::region(lat, lon)).unwrap_or_default()
    }
}

/// Days between two `YYYY-MM-DD` dates (`b - a`), for the UI's "today" marker.
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    Some((b.parse::<NaiveDate>().ok()? - a.parse::<NaiveDate>().ok()?).num_days())
}

#[allow(dead_code)]
fn _assert_send_sync()
where
    SnowTrack: Send + Sync,
{
    let _ = Duration::zero();
}

#[cfg(test)]
mod tests;

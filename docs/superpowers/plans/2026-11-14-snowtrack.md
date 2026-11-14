# SnowTrack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An Android app showing past and forecast snowfall for user-chosen ski resorts as cards, each opening a detail page with radar.

**Architecture:** A Rust library (`core/`) does all fetching, parsing, maths and storage and exposes one `SnowTrack` object through UniFFI. A Kotlin/Compose app (`android/`) renders what that object returns. GitHub Actions builds the APK.

**Tech Stack:** Rust (ureq, rustls, serde, quick-xml, chrono, uniffi), Kotlin 1.9.24, AGP 8.5.2, Jetpack Compose (BOM 2024.06.00), Material 3, MapLibre Android 11, cargo-ndk.

**Spec:** `docs/superpowers/specs/2026-11-14-snowtrack-design.md`

## Global Constraints

- Work on branch `snowtrack` of `HammerJones/MGBW-RFID`.
- Android package `com.hammerjones.snowtrack`; minSdk 26, compile/target SDK 34, ABI `arm64-v8a`.
- No API keys. Every HTTP request sends `User-Agent: SnowTrack/0.1 (github.com/HammerJones)`.
- Missing data is `None`/"—", never zero and never estimated.
- Rust is stored in metric (mm, cm, °C, metres); conversion happens only at the display edge (`Units`).
- All network code takes `&dyn Http`; no test touches the network.

## Review Focus

1. A resort with no SNOTEL station within 60 km (e.g. Midwest/East): history is `None`, forecast still shows.
2. NWS `validTime` intervals that span midnight or several days (`PT6H`, `P1D`, `P1DT12H`): amounts are split across local days in proportion to hours.
3. One source failing (HTTP 500, timeout, malformed JSON): the other sources still populate the snapshot and the failure is listed in `snapshot.problems`.
4. Offline refresh: the cached snapshot is returned with `stale = true` rather than an error.
5. SNOTEL depth sensor noise (null days, negative deltas, one-day spikes > 100 cm): nulls are skipped, negatives count as 0 new snow, spikes are dropped.

---

### Task 1: Clean slate and Rust core skeleton

**Files:** delete old `src/`, `Cargo.*`; create `Cargo.toml` (workspace, member `core`), `core/Cargo.toml`, `core/src/{lib,model,http}.rs`, `.gitignore`, `README.md`.

**Produces:**
- `model.rs`: `Resort { id, name, state, lat, lon, base_elev_m, summit_elev_m }`, `DaySnow { date: String /*YYYY-MM-DD local*/, snow_cm: Option<f64>, depth_cm: Option<f64>, temp_hi_c: Option<f64>, temp_lo_c: Option<f64>, is_forecast: bool }`, `Alert { event, headline, severity, ends }`, `NewsItem { title, source, url, published, reported_snow_cm: Option<f64> }`, `StationInfo { name, triplet, distance_km, elev_m }`, `Snapshot { resort, days: Vec<DaySnow>, last_24h_cm, next_24h_cm, next_72h_cm, next_7d_cm, depth_cm, station: Option<StationInfo>, alerts, news, outlook: Option<String>, radar_station: Option<String>, fetched_at: String, stale: bool, problems: Vec<String> }` — all `serde` + `uniffi::Record`.
- `http.rs`: `trait Http { fn get(&self, url: &str) -> Result<String, HttpError>; }`, `UreqHttp` (15 s timeout, User-Agent above), `FakeHttp` (test-only, URL-substring → body or error).

- [ ] Test `fake_http_matches_by_substring` and `fake_http_unmatched_is_error`; implement; `cargo test -p snowtrack-core`; commit.

### Task 2: Interval parsing and day bucketing

**Files:** `core/src/interval.rs`

**Produces:** `parse_valid_time(s) -> Option<(DateTime<Utc>, Duration)>`; `split_by_local_day(start, dur, value, tz_offset_min) -> Vec<(NaiveDate, f64)>`.

- [ ] Tests: `"2026-11-14T06:00:00+00:00/PT6H"` → 6 h; `P1DT12H` → 36 h; `P7D` → 168 h; garbage → `None`; a 6 h/6.0 mm interval starting 03:00 UTC at offset −420 splits 4.0 to the earlier local day and 2.0 to the later. Implement; commit.

### Task 3: NWS forecast, alerts, outlook

**Files:** `core/src/nws.rs`, fixtures `core/tests/fixtures/nws_{points,grid,alerts,afd_list,afd}.json`

**Produces:** `fetch_point(http, lat, lon) -> Result<PointInfo { office, grid_url, radar_station, tz }>`; `fetch_forecast_days(http, &PointInfo, today) -> Result<Vec<DaySnow>>` (7 forecast days, snow summed per local day, hi/lo from `temperature`); `fetch_alerts(http, lat, lon) -> Result<Vec<Alert>>` (only events containing Winter, Snow, Blizzard, Avalanche, Ice, Freeze); `fetch_outlook(http, office) -> Result<Option<String>>` (the `.SHORT TERM`/`.LONG TERM`/`.DISCUSSION` sections of the latest AFD, max 1500 chars).

- [ ] Tests on fixtures: snowfall mm → cm per day; `null` values skipped; a day with no snowfall entries has `snow_cm == Some(0.0)` only if the grid covered it, else `None`; non-winter alerts filtered out; AFD section extraction. Review Focus 2 test lives here. Implement; commit.

### Task 4: SNOTEL history

**Files:** `core/src/snotel.rs`, `core/src/geo.rs`, fixtures `awdb_{stations,data}.json`

**Produces:** `geo::haversine_km(a_lat, a_lon, b_lat, b_lon) -> f64`; `nearest_station(http, lat, lon) -> Result<Option<StationInfo>>` (AWDB `/stations?stationTriplets=*:*:SNTL&activeOnly=true`, within 60 km); `fetch_history(http, &StationInfo, today, days: u32) -> Result<Vec<DaySnow>>` (daily `SNWD`, inches → cm, new snow = positive delta).

- [ ] Tests: Denver→Vail distance 117–125 km; nearest picks the closer of two; none within 60 km → `None`; deltas 100,110,105,null,120 → new snow `None,10,0,None,None` (gap resets delta); a 150 cm one-day spike is dropped. Review Focus 1 and 5. Implement; commit.

### Task 5: News

**Files:** `core/src/news.rs`, fixture `news.xml`

**Produces:** `news_url(resort_name) -> String`; `parse_rss(xml) -> Vec<NewsItem>`; `extract_snow_cm(title) -> Option<f64>`; `fetch_news(http, &Resort) -> Result<Vec<NewsItem>>` (newest first, max 15, deduped by title).

- [ ] Tests: `"Vail reports 14 inches overnight"` → 35.56; `"8-12 inches expected"` → upper bound 30.48; `"2 feet of snow"` → 60.96; `"30 cm"` → 30.0; `"Lift 14 opens"` → `None`; RSS with CDATA titles and entity-escaped links parses; malformed XML → empty list. Implement; commit.

### Task 6: Catalog, storage, refresh, UniFFI surface

**Files:** `core/src/{catalog,store,summary,lib}.rs`, `core/resorts.json`, `core/src/bin/snowtrack.rs`, `core/uniffi-bindgen.rs`

**Produces (the whole Kotlin-facing API):**
```rust
#[uniffi::export] impl SnowTrack {
  #[uniffi::constructor] fn new(data_dir: String) -> Arc<Self>;
  fn search_catalog(&self, query: String) -> Vec<Resort>;   // name/state substring, max 30
  fn resorts(&self) -> Vec<Resort>;
  fn add_resort(&self, resort: Resort) -> Result<(), SnowError>;       // duplicate id → error
  fn add_custom(&self, name: String, lat: f64, lon: f64) -> Result<Resort, SnowError>; // US bounds check
  fn remove_resort(&self, id: String);
  fn cached(&self, id: String) -> Option<Snapshot>;
  fn refresh(&self, id: String) -> Result<Snapshot, SnowError>;
  fn radar_tile_template(&self) -> String;                  // WMS URL with {bbox-epsg-3857}
}
```
`summary::build(resort, history, forecast, …, now) -> Snapshot` computes the headline totals. `refresh` runs every source, records failures in `problems`, saves the snapshot, and on total failure returns the cache with `stale = true`.

- [ ] Tests: catalog JSON loads, ids unique, all coordinates inside US bounds; search "vail" finds Vail; store round-trips through a temp dir and survives a corrupt file; `refresh` with a `FakeHttp` full set yields 14 history + 7 forecast days and correct totals; Review Focus 3 and 4. Implement; run CLI `snowtrack vail` shape check with fixtures; generate Kotlin bindings on host to prove the UniFFI surface compiles; commit.

### Task 7: Android app

**Files:** `android/{settings,build}.gradle.kts`, `android/gradle.properties`, `android/app/build.gradle.kts`, `AndroidManifest.xml`, and under `com/hammerjones/snowtrack/`: `MainActivity.kt` (nav host), `SnowViewModel.kt`, `ui/HomeScreen.kt` (cards, pull-to-refresh, empty state), `ui/ResortCard.kt`, `ui/SnowChart.kt` (Canvas bar chart: history solid, forecast lighter, "today" divider, depth line), `ui/AddResortScreen.kt` (search + custom), `ui/ResortScreen.kt` (radar map, chart, table, alerts, outlook, news), `ui/RadarMap.kt` (MapLibre + WMS raster layer, play/refresh), `ui/Format.kt` (units), `ui/Theme.kt`.

**Consumes:** the `SnowTrack` API from Task 6 via generated `uniffi/snowtrack_core/snowtrack_core.kt`.

- [ ] Unit test `FormatTest` (cm→in rounding, "—" for null, trace amounts `< 0.1"`); implement screens; this task is verified by the CI build in Task 8 since Gradle cannot run locally.

### Task 8: CI build and delivery

**Files:** `.github/workflows/snowtrack.yml`, `README.md`

- [ ] Workflow (push to any branch, manual dispatch): `cargo test`, `cargo ndk` arm64 build, UniFFI Kotlin generation, `gradle testReleaseUnitTest assembleRelease`, cached signing key, upload `SnowTrack.apk`. Push, watch the run with `gh run watch`, fix until green; README says how to download and install the APK.

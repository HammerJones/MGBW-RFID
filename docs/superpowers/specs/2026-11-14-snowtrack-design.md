# SnowTrack — design

## Intent

An Android app for checking how much snow has fallen and how much is coming at
the ski resorts Sean cares about, built from free NOAA data plus news reports.
One glance at the home screen should answer "where is it snowing?"; one tap
should show the detail and the radar.

## Goals

- Home screen is a list of cards, one per resort the user has added.
- Each card shows a graph (past + forecast snowfall by day) and headline
  numbers (last 24 h, next 24 h, next 7 days, snow depth).
- Tapping a card opens a resort page: live radar on a map, a larger chart,
  a day-by-day table, active NWS winter alerts, and news.
- The user adds resorts by searching a bundled catalog, or by entering a
  custom name and coordinates. Resorts can be removed.
- Last good data is cached and shown when offline, marked with its age.
- As much logic as possible lives in Rust.

## Non-goals

- Resorts outside the United States (NOAA/NWS coverage only).
- Accounts, sync, push notifications, home-screen widgets.
- Lift status, trail maps, or resort-reported snow totals (no free source).
- iOS or desktop UI. (The Rust core does get a small desktop CLI for testing.)

## Anti-goals

- No API keys or paid services: the app must work straight after install.
- No invented numbers: when a source is missing, show "—", never a guess.

## Constraints

- The development workspace cannot download the Android SDK, so the APK is
  built and checked by GitHub Actions; only the Rust core is tested locally.
- `api.weather.gov` requires a `User-Agent` header identifying the app.
- The repo also hosts the budget app on the `budget-app` branch. SnowTrack
  lives on its own branch and must not touch `budget-app` or `master`.

## Approaches

1. **Rust core + Kotlin/Jetpack Compose UI over UniFFI (chosen).** Rust owns
   networking, parsing, snowfall maths, catalog and storage; Kotlin owns only
   screens, charts and the map. Native scrolling, map and system look.
2. **Pure Rust UI (egui/Slint/Dioxus).** More Rust, but non-native feel on
   Android, weak map support for radar tiles, awkward keyboard and back
   handling. Rejected on UI quality.
3. **Kotlin for everything.** Simplest build, but ignores the "as much Rust as
   possible" requirement. Rejected.

Sean approved approach 1.

## Data sources (builder's calls)

| Need | Source |
|---|---|
| Forecast snowfall, temperature, snow level | NWS API: `/points/{lat},{lon}` then `forecastGridData` (`snowfallAmount`, `temperature`, `snowLevel`, `quantitativePrecipitation`) |
| Winter alerts | NWS API: `/alerts/active?point={lat},{lon}` |
| Forecaster's written outlook | NWS API: latest `AFD` product for the resort's forecast office |
| Past snowfall and snow depth | USDA NRCS AWDB REST API, nearest SNOTEL station: daily `SNWD` and `WTEQ` |
| News reports | Google News RSS search for `"<resort name>" snow`, last 7 days; snowfall amounts mentioned in headlines are extracted and shown as "reported" |
| Radar | NOAA MRMS base reflectivity WMS (`opengeo.ncep.noaa.gov`, layer `conus_bref_qcd`) over an OpenFreeMap base map in MapLibre |

"New snow" for a past day is the positive day-over-day change in SNOTEL snow
depth; this under-reads when snow settles and is labelled "at nearest SNOTEL
station (<name>, <distance>)" in the UI.

## How it is built (builder's calls)

- Rust crate `snowtrack-core` in `core/`: blocking HTTP via `ureq` + rustls,
  `serde`/`quick-xml` parsing, JSON file storage in the app's files dir,
  UniFFI proc-macro bindings. All network access goes through an `Http` trait
  so every parser and the whole refresh flow are tested against fixtures.
- Android app in `android/`, package `com.hammerjones.snowtrack`, Compose +
  Material 3, charts drawn with Compose `Canvas` (no chart library), MapLibre
  for the radar map. Rust calls run on `Dispatchers.IO`.
- GitHub Actions builds arm64 `.so` with `cargo ndk`, generates Kotlin
  bindings, builds a signed APK and uploads it as an artifact — same pattern
  as the budget app's workflow, including the cached signing key.
- Units: inches and °F by default, with a metric toggle.

## Added 2026-11-14: snow notifications

Sean asked for a bell on each card. With the bell on, the app checks that
resort in the background every 3 hours (WorkManager, needs a connection) and
posts a notification when the 72-hour forecast reaches the threshold in
Settings (default 6 in), or when NWS issues a new winter alert. One
notification per storm: it repeats only if the forecast grows by 10 cm or
more, and re-arms once the forecast falls below half the threshold. The
decision logic lives in Rust (`core/src/notify.rs`); Kotlin only posts.
"Notifications" is therefore no longer a non-goal. The app now lives in
`HammerJones/MGBW-RFID` (old contents removed) rather than `rfid`.

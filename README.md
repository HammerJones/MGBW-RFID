# SnowTrack

An Android app that tracks and forecasts snowfall at the ski resorts you pick.

- **Home**: one card per resort with a snow-by-day chart (what fell and what's forecast) and the headline numbers.
- **Resort page**: live NOAA radar on a topo map with a playable loop, totals, a day-by-day table, winter weather alerts, the local forecaster's outlook, and recent news with any snowfall amounts pulled from the headlines.
- **Bell**: tap the bell on a card to be notified when a big snowfall is forecast or a winter alert is issued for that resort. The amount that counts as "big" is in Settings (default 6").
- Add resorts from a built-in list of about 140 US ski areas, or any US location by coordinates.

No accounts or API keys. US only, because the data is NOAA's.

## Install

Every push builds an APK. On GitHub open **Actions**, pick the latest **Build** run, download **snowtrack-android**, unzip it and open `SnowTrack.apk` on your phone (allow installs from your browser or file manager when asked). 64-bit ARM phones, Android 8 or newer.

## Where the data comes from

| What | Source |
|---|---|
| Forecast snow and temperatures, alerts, forecaster's outlook | National Weather Service API (`api.weather.gov`) |
| Past snowfall and snow depth | Nearest SNOTEL station within 60 km (USDA NRCS). Past snow is the daily rise in snow depth, so it can read low when snow settles. Resorts with no station nearby show the forecast only. |
| News | Google News search for the resort, last 7 days |
| Radar | NOAA MRMS base reflectivity mosaic |
| Base map | USGS The National Map |

Resort coordinates and elevations in `core/resorts.csv` were entered by hand and are approximate; corrections welcome.

## How it's built

- `core/` — Rust. All networking, parsing, snowfall maths, notification decisions and storage. Exposed to Android through [UniFFI](https://mozilla.github.io/uniffi-rs/).
- `android/` — Kotlin + Jetpack Compose. Screens, charts and the MapLibre radar map only.

```sh
cargo test -p snowtrack-core            # run the core's tests
cargo run -p snowtrack-core --bin snowtrack -- vail   # try the core from a terminal
./scripts/build-android.sh && (cd android && gradle assembleRelease)   # local APK; needs the Android SDK/NDK and cargo-ndk
```

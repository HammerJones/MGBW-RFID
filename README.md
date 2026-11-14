# SnowTrack

An Android app that tracks and forecasts snowfall at the ski resorts you pick.

- **Home**: one card per resort with a snow-by-day chart (what fell and what's forecast) and the headline numbers.
- **Resort page**: live NOAA radar on a topo map with a playable loop, totals, a day-by-day table, winter weather alerts, the local forecaster's outlook, and recent news with any snowfall amounts pulled from the headlines.
- **Bell**: tap the bell on a card to be notified when a big snowfall is forecast or a winter alert is issued for that resort. The amount that counts as "big" is in Settings (default 6").
- **Long range**: charts scroll sideways out to 16 days. Days past the Weather Service forecast come from NOAA's GFS model and are marked as a rough guide.
- **Updates itself**: when a newer build is published, a banner on the home screen downloads it and opens Android's installer. Settings has a manual check.
- Add resorts from a built-in list of about 140 US ski areas, or any US location by coordinates.

No accounts or API keys. US only, because the data is NOAA's.

## Install

Every push to `snowtrack` publishes a release. On GitHub open **Releases**, download `SnowTrack.apk` from the newest one and open it on your phone (allow installs from your browser or file manager when asked). 64-bit ARM phones, Android 8 or newer. After that the app updates itself.

Updates only install over a build signed with the same key. The build keeps its key in the Actions cache, which GitHub clears after a week without builds; to make it permanent, add a repository secret `SNOWTRACK_KEYSTORE_B64` (see `.github/workflows/build.yml`). If an update ever refuses to install, uninstall SnowTrack and install the new APK.

## Where the data comes from

| What | Source |
|---|---|
| Forecast snow and temperatures, alerts, forecaster's outlook | National Weather Service API (`api.weather.gov`) |
| Past snowfall and snow depth | Nearest SNOTEL station within 60 km (USDA NRCS). Past snow is the daily rise in snow depth, so it can read low when snow settles. Resorts with no station nearby show the forecast only. |
| Long-range days | NOAA GFS model via Open-Meteo (free for non-commercial use) |
| News | Google News search for the resort, last 7 days |
| Radar | NOAA MRMS base reflectivity mosaic |
| Base map | USGS The National Map |

Resort coordinates and elevations in `core/resorts.csv` were entered by hand and are approximate; corrections welcome.

## How it's built

- `core/` — Rust. All networking, parsing, snowfall maths, notification decisions and storage. Exposed to Android through [UniFFI](https://mozilla.github.io/uniffi-rs/).
- `android/` — Kotlin + Jetpack Compose. Screens, charts and the osmdroid radar map only.

```sh
cargo test -p snowtrack-core            # run the core's tests
cargo run -p snowtrack-core --bin snowtrack -- vail   # try the core from a terminal
./scripts/build-android.sh && (cd android && gradle assembleRelease)   # local APK; needs the Android SDK/NDK and cargo-ndk
```

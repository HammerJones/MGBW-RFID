//! End-to-end tests of the refresh flow against canned responses.

use super::*;
use crate::http::fake::FakeHttp;
use crate::news::tests::RSS;
use crate::nws::tests::{alerts_json, grid_json, point_json, AFD};
use crate::snotel::tests::{data_json, stations_json};
use serde_json::json;

fn now() -> DateTime<Utc> {
    // Noon Mountain time on Saturday 14 November 2026.
    DateTime::parse_from_rfc3339("2026-11-14T19:00:00Z").unwrap().with_timezone(&Utc)
}

fn history_body() -> String {
    // End-of-day depths in inches, 6 Nov – 13 Nov.
    data_json(&[
        ("2026-11-06", Some(10.0)),
        ("2026-11-07", Some(10.0)),
        ("2026-11-08", Some(12.0)),
        ("2026-11-09", Some(11.0)),
        ("2026-11-10", None),
        ("2026-11-11", Some(11.0)),
        ("2026-11-12", Some(15.0)),
        ("2026-11-13", Some(21.0)),
    ])
}

fn full_http() -> FakeHttp {
    FakeHttp::new()
        .ok("/points/", point_json())
        .ok("/gridpoints/", grid_json())
        .ok("/alerts/active", alerts_json())
        .ok("/products/types/AFD", json!({"@graph": [{"@id": "https://api.weather.gov/products/abc"}]}).to_string())
        .ok("/products/abc", json!({"productText": AFD}).to_string())
        .ok("/stations?", stations_json())
        .ok("/data?", history_body())
        .ok("news.google.com", RSS)
}

fn app(dir: &std::path::Path, http: FakeHttp) -> (SnowTrack, Arc<FakeHttp>) {
    let http = Arc::new(http);
    (SnowTrack::with_parts(dir.to_str().unwrap(), http.clone(), Box::new(now)), http)
}

fn add_vail(st: &SnowTrack) -> String {
    let vail = catalog::search("vail", 1).remove(0);
    st.add_resort(vail.clone()).unwrap();
    vail.id
}

fn close(a: Option<f64>, b: f64) -> bool {
    a.map_or(false, |a| (a - b).abs() < 1e-6)
}

#[test]
fn refresh_builds_a_complete_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let (st, http) = app(dir.path(), full_http());
    let id = add_vail(&st);
    let s = st.refresh(id.clone()).unwrap();

    assert!(s.problems.is_empty(), "{:?}", s.problems);
    assert!(!s.stale);
    assert_eq!(s.days.len(), 14);
    assert_eq!(s.days[0].date, "2026-11-07");
    assert_eq!(s.days[6].date, "2026-11-13");
    assert!(!s.days[6].is_forecast && s.days[7].is_forecast);
    assert_eq!(s.days[7].date, "2026-11-14");
    assert_eq!(s.days[13].date, "2026-11-20");

    // History: 0, 2", 0 (settled), gap, gap, 4", 6".
    let snow: Vec<Option<f64>> = s.days[..7].iter().map(|d| d.snow_cm).collect();
    assert_eq!(snow[0], Some(0.0));
    assert!(close(snow[1], 5.08));
    assert_eq!(snow[2], Some(0.0));
    assert_eq!(snow[3], None);
    assert_eq!(snow[4], None);
    assert!(close(snow[5], 10.16));
    assert!(close(snow[6], 15.24));
    assert!(close(s.last_24h_cm, 15.24));
    assert!(close(s.last_72h_cm, 25.4));
    assert!(close(s.last_7d_cm, 30.48));
    assert!(close(s.depth_cm, 53.34));

    assert!(close(s.next_24h_cm, 4.8));
    assert!(close(s.next_72h_cm, 6.0));
    assert!(close(s.next_7d_cm, 6.0));
    assert!(close(s.days[8].snow_cm, 4.0));

    assert_eq!(s.station.as_ref().unwrap().name, "Vail Mountain");
    assert_eq!(s.alerts.len(), 2);
    assert_eq!(s.news.len(), 2);
    assert!(s.outlook.unwrap().contains("Heavy mountain snow"));
    assert_eq!(s.radar_station.as_deref(), Some("KGJX"));
    assert_eq!(s.fetched_at_epoch_s, now().timestamp());

    // It is saved, and shows on the home screen.
    assert_eq!(st.snapshot(id.clone()).unwrap().days.len(), 14);
    assert!(st.cards()[0].snapshot.is_some());

    // Second refresh reuses the cached grid location and station.
    st.refresh(id).unwrap();
    assert_eq!(http.count("/points/"), 1);
    assert_eq!(http.count("/stations?"), 1);
    assert_eq!(http.count("/gridpoints/"), 2);
}

#[test]
fn history_request_asks_for_end_of_day_depth() {
    let dir = tempfile::tempdir().unwrap();
    let (st, http) = app(dir.path(), full_http());
    let id = add_vail(&st);
    st.refresh(id).unwrap();
    let log = http.log.lock().unwrap();
    let url = log.iter().find(|u| u.contains("/data?")).unwrap();
    assert!(url.contains("stationTriplets=842:CO:SNTL"), "{url}");
    assert!(url.contains("elements=SNWD&duration=DAILY&periodRef=END"), "{url}");
    assert!(url.contains("beginDate=2026-11-06&endDate=2026-11-14"), "{url}");
}

#[test]
fn resort_with_no_station_nearby_still_gets_a_forecast() {
    let dir = tempfile::tempdir().unwrap();
    let (st, http) = app(dir.path(), full_http());
    let killington = catalog::search("killington", 1).remove(0);
    st.add_resort(killington.clone()).unwrap();
    let s = st.refresh(killington.id.clone()).unwrap();
    assert!(s.problems.is_empty(), "{:?}", s.problems);
    assert_eq!(s.station, None);
    assert_eq!(s.days.len(), 7);
    assert!(s.days.iter().all(|d| d.is_forecast));
    assert_eq!((s.last_24h_cm, s.last_7d_cm, s.depth_cm), (None, None, None));
    assert!(close(s.next_24h_cm, 4.8));
    assert_eq!(http.count("/data?"), 0);
    // "None nearby" is remembered, even after a restart, without re-reading the list.
    std::fs::remove_file(dir.path().join("stations.json")).unwrap();
    let (st, http2) = app(dir.path(), full_http());
    assert_eq!(st.refresh(killington.id).unwrap().station, None);
    assert_eq!(http.count("/stations?"), 1);
    assert_eq!(http2.count("/stations?"), 0);
}

#[test]
fn one_failing_source_does_not_sink_the_others() {
    for (pattern, word) in [("/gridpoints/", "forecast"), ("/data?", "SNOTEL"), ("/alerts/active", "alerts"), ("news.google.com", "News"), ("/products/types/AFD", "discussion")] {
        let dir = tempfile::tempdir().unwrap();
        let (st, _) = app(dir.path(), FakeHttp::new().fail(pattern, "HTTP 500").ok("/points/", point_json()).ok("/gridpoints/", grid_json()).ok("/alerts/active", alerts_json()).ok("/products/types/AFD", json!({"@graph": [{"@id": "https://api.weather.gov/products/abc"}]}).to_string()).ok("/products/abc", json!({"productText": AFD}).to_string()).ok("/stations?", stations_json()).ok("/data?", history_body()).ok("news.google.com", RSS));
        let id = add_vail(&st);
        let s = st.refresh(id).unwrap();
        assert_eq!(s.problems.len(), 1, "{pattern}: {:?}", s.problems);
        assert!(s.problems[0].contains(word) && s.problems[0].contains("HTTP 500"), "{:?}", s.problems);
        assert!(!s.stale);
        if pattern == "/gridpoints/" {
            assert_eq!(s.next_24h_cm, None);
            assert_eq!(s.days.len(), 7);
            assert!(close(s.last_24h_cm, 15.24));
        } else {
            assert!(close(s.next_24h_cm, 4.8), "{pattern}");
        }
        if pattern == "/data?" {
            assert_eq!(s.last_24h_cm, None);
            assert_eq!(s.days.len(), 7);
            assert_eq!(s.station.as_ref().unwrap().name, "Vail Mountain");
        }
    }
}

#[test]
fn malformed_responses_are_reported_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let http = FakeHttp::new().ok("/gridpoints/", "<html>Service Unavailable</html>").ok("/data?", "[]").ok("/alerts/active", "{}");
    let (st, _) = app(dir.path(), http.ok("/points/", point_json()).ok("/products/types/AFD", "{}").ok("/stations?", stations_json()).ok("news.google.com", "not xml at all"));
    let id = add_vail(&st);
    let s = st.refresh(id).unwrap();
    assert_eq!(s.next_24h_cm, None);
    assert!(s.news.is_empty());
    assert!(s.alerts.is_empty());
    assert_eq!(s.outlook, None);
    // Empty SNOTEL data: seven history days, all unknown.
    assert_eq!(s.days.len(), 7);
    assert!(s.days.iter().all(|d| d.snow_cm.is_none() && d.depth_cm.is_none()));
    assert_eq!(s.problems.len(), 2, "{:?}", s.problems);
}

#[test]
fn offline_refresh_returns_saved_data_marked_stale() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let (st, _) = app(dir.path(), full_http());
        let id = add_vail(&st);
        st.refresh(id.clone()).unwrap();
        id
    };
    let (st, _) = app(dir.path(), FakeHttp::new());
    let s = st.refresh(id.clone()).unwrap();
    assert!(s.stale);
    assert_eq!(s.days.len(), 14);
    assert!(!s.problems.is_empty());
    // The saved copy itself is not overwritten with the stale flag.
    assert!(!st.snapshot(id).unwrap().stale);
}

#[test]
fn offline_with_nothing_saved_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _) = app(dir.path(), FakeHttp::new());
    let id = add_vail(&st);
    assert!(matches!(st.refresh(id), Err(SnowError::Network { .. })));
    assert_eq!(st.refresh("nope".into()), Err(SnowError::NotFound));
}

#[test]
fn stale_grid_address_is_looked_up_again() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let (st, _) = app(dir.path(), full_http());
        let id = add_vail(&st);
        st.refresh(id.clone()).unwrap();
        id
    };
    let moved = point_json().replace("GJT/173,120", "BOU/20,57");
    let http = FakeHttp::new().fail("GJT/173,120", "HTTP 404").ok("/points/", moved).ok("BOU/20,57", grid_json()).ok("/alerts/active", alerts_json()).ok("/data?", history_body()).ok("news.google.com", RSS).ok("/products/types/AFD", "{\"@graph\":[]}");
    let (st, http) = app(dir.path(), http);
    let s = st.refresh(id.clone()).unwrap();
    assert!(close(s.next_24h_cm, 4.8), "{:?}", s.problems);
    st.refresh(id).unwrap();
    assert_eq!(http.count("GJT/173,120"), 1);
}

#[test]
fn adding_and_removing_resorts() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _) = app(dir.path(), full_http());
    let id = add_vail(&st);
    let vail = st.cards()[0].resort.clone();
    assert!(matches!(st.add_resort(vail), Err(SnowError::Invalid { .. })));
    assert!(!st.search_catalog("vail".into()).iter().any(|r| r.id == id));

    let custom = st.add_custom("  Berthoud   Pass ".into(), 39.798, -105.777).unwrap();
    assert_eq!(custom.name, "Berthoud Pass");
    assert_eq!(custom.id, "berthoud-pass-custom");
    assert_eq!(st.add_custom("Berthoud Pass".into(), 39.8, -105.78).unwrap().id, "berthoud-pass-custom-2");
    assert!(matches!(st.add_custom("Whistler".into(), 50.11, -122.95), Err(SnowError::Invalid { .. })));
    assert!(matches!(st.add_custom("".into(), 39.8, -105.7), Err(SnowError::Invalid { .. })));
    assert!(matches!(st.add_custom("Nowhere".into(), f64::NAN, -105.7), Err(SnowError::Invalid { .. })));

    st.refresh(id.clone()).unwrap();
    st.remove_resort(id.clone());
    assert_eq!(st.cards().len(), 2);
    assert!(st.snapshot(id).is_none());

    // Survives a restart.
    let (again, _) = app(dir.path(), full_http());
    assert_eq!(again.cards().iter().map(|c| c.resort.name.clone()).collect::<Vec<_>>(), vec!["Berthoud Pass", "Berthoud Pass"]);
}

#[test]
fn bell_notifies_once_per_storm_and_alert() {
    let dir = tempfile::tempdir().unwrap();
    let (st, http) = app(dir.path(), full_http());
    let id = add_vail(&st);
    // Bell off: nothing is fetched at all.
    assert!(st.check_notifications().is_empty());
    assert_eq!(http.count("/gridpoints/"), 0);

    st.set_notify(id.clone(), true);
    assert!(st.cards()[0].notify);
    st.set_settings(Settings { metric: false, notify_threshold_cm: 5.08 });
    let n = st.check_notifications();
    let titles: Vec<_> = n.iter().map(|x| x.title.as_str()).collect();
    assert_eq!(titles, vec!["2\" of snow coming to Vail", "Winter Storm Warning for Vail", "Avalanche Warning for Vail"]);
    assert!(st.check_notifications().is_empty());

    // Switching the bell off and on starts fresh.
    st.set_notify(id.clone(), false);
    st.set_notify(id, true);
    assert_eq!(st.check_notifications().len(), 3);
}

#[test]
fn settings_are_clamped_and_saved() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _) = app(dir.path(), FakeHttp::new());
    assert_eq!(st.settings(), Settings::default());
    st.set_settings(Settings { metric: true, notify_threshold_cm: 9999.0 });
    assert_eq!(st.settings().notify_threshold_cm, 150.0);
    st.set_settings(Settings { metric: true, notify_threshold_cm: f64::NAN });
    assert_eq!(st.settings().notify_threshold_cm, 15.24);
    let (again, _) = app(dir.path(), FakeHttp::new());
    assert!(again.settings().metric);
}

#[test]
fn history_totals_handle_gaps() {
    let day = |s: Option<f64>, d: Option<f64>| DaySnow { date: "x".into(), snow_cm: s, depth_cm: d, temp_hi_c: None, temp_lo_c: None, is_forecast: false };
    assert_eq!(history_totals(&[]), (None, None, None, None));
    let h = [day(Some(1.0), Some(50.0)), day(None, None), day(None, None), day(None, None)];
    assert_eq!(history_totals(&h), (None, None, Some(1.0), Some(50.0)));
}

#[test]
fn radar_helpers() {
    let dir = tempfile::tempdir().unwrap();
    let (st, _) = app(dir.path(), FakeHttp::new());
    assert!(st.radar_tile_template(39.6, -106.4, None).contains("{bbox-epsg-3857}"));
    assert!(st.radar_times(39.6, -106.4).is_empty());
    let caps = "<Dimension name=\"time\" default=\"x\">2026-11-11T07:46:13.000Z,2026-11-11T09:43:58.000Z</Dimension>";
    let (st, http) = app(dir.path(), FakeHttp::new().ok("/alaska/alaska_bref_qcd/ows?service=WMS&version=1.3.0&request=GetCapabilities", caps));
    assert_eq!(st.radar_times(60.97, -149.1).len(), 2);
    assert_eq!(http.count("alaska"), 1);
    assert_eq!(days_between("2026-11-14", "2026-11-16"), Some(2));
    assert_eq!(days_between("bad", "2026-11-16"), None);
}

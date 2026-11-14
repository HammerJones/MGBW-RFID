//! Decides when a resort with its bell switched on deserves a notification.

use crate::model::{Snapshot, SnowNotice};
use serde::{Deserialize, Serialize};

/// What we have already told the user about, per resort.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NotifyState {
    /// The 72 h forecast we last notified about, if a storm is "open".
    pub storm_cm: Option<f64>,
    /// Alerts already announced, as `event|ends`.
    pub alerts: Vec<String>,
}

/// A storm we already announced is re-announced only if the forecast grows by this much.
const REPEAT_GROWTH_CM: f64 = 10.0;

fn amount(cm: f64, metric: bool) -> String {
    if metric {
        format!("{:.0} cm", cm)
    } else {
        format!("{:.0}\"", cm / 2.54)
    }
}

pub fn decide(state: &NotifyState, snap: &Snapshot, threshold_cm: f64, metric: bool) -> (Vec<SnowNotice>, NotifyState) {
    let mut next = state.clone();
    let mut notices = Vec::new();
    if snap.stale {
        return (notices, next);
    }
    let name = crate::news::short_name(&snap.resort.name);
    let notice = |tag: String, title: String, body: String| SnowNotice {
        resort_id: snap.resort.id.clone(),
        resort_name: snap.resort.name.clone(),
        tag,
        title,
        body,
    };

    if let Some(cm) = snap.next_72h_cm {
        let big = cm >= threshold_cm;
        match (big, state.storm_cm) {
            (true, None) => {
                notices.push(notice("storm".into(), format!("{} of snow coming to {name}", amount(cm, metric)), storm_body(snap, metric)));
                next.storm_cm = Some(cm);
            }
            (true, Some(prev)) if cm >= prev + REPEAT_GROWTH_CM => {
                notices.push(notice(
                    "storm".into(),
                    format!("{name} forecast up to {}", amount(cm, metric)),
                    format!("Was {}. {}", amount(prev, metric), storm_body(snap, metric)),
                ));
                next.storm_cm = Some(cm);
            }
            // The storm has passed or fizzled: arm for the next one.
            (false, Some(_)) if cm < threshold_cm * 0.5 => next.storm_cm = None,
            _ => {}
        }
    }

    let current: Vec<String> =
        snap.alerts.iter().map(|a| format!("{}|{}", a.event, a.ends.clone().unwrap_or_default())).collect();
    for (alert, key) in snap.alerts.iter().zip(&current) {
        if !state.alerts.contains(key) {
            notices.push(notice(format!("alert:{}", alert.event), format!("{} for {name}", alert.event), alert.headline.clone()));
        }
    }
    // Only forget alerts when we actually know the current list.
    if !snap.problems.iter().any(|p| p.contains("alerts")) {
        next.alerts = current;
    } else {
        for key in current {
            if !next.alerts.contains(&key) {
                next.alerts.push(key);
            }
        }
    }
    (notices, next)
}

fn storm_body(snap: &Snapshot, metric: bool) -> String {
    let mut parts = vec!["Next 72 hours.".to_string()];
    if let Some(d) = snap.next_24h_cm {
        parts.push(format!("{} in the next 24.", amount(d, metric)));
    }
    if let Some(best) = snap
        .days
        .iter()
        .filter(|d| d.is_forecast)
        .filter_map(|d| d.snow_cm.map(|s| (s, &d.date)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|b| b.0 > 0.0)
    {
        parts.push(format!("Biggest day: {} ({}).", weekday(best.1), amount(best.0, metric)));
    }
    parts.join(" ")
}

fn weekday(date: &str) -> String {
    date.parse::<chrono::NaiveDate>().map(|d| d.format("%A").to_string()).unwrap_or_else(|_| date.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Alert, DaySnow};

    fn snap(next72: Option<f64>, alerts: Vec<Alert>) -> Snapshot {
        Snapshot {
            resort: crate::news::tests::vail(),
            days: vec![
                DaySnow { date: "2026-11-14".into(), snow_cm: Some(5.0), depth_cm: None, temp_hi_c: None, temp_lo_c: None, is_forecast: true },
                DaySnow { date: "2026-11-15".into(), snow_cm: Some(20.0), depth_cm: None, temp_hi_c: None, temp_lo_c: None, is_forecast: true },
            ],
            last_24h_cm: None, last_72h_cm: None, last_7d_cm: None,
            next_24h_cm: Some(5.08), next_72h_cm: next72, next_7d_cm: next72,
            depth_cm: None, station: None, alerts, news: vec![], outlook: None, radar_station: None,
            fetched_at_epoch_s: 0, stale: false, problems: vec![],
        }
    }

    fn warning() -> Alert {
        Alert { event: "Winter Storm Warning".into(), headline: "Heavy snow expected".into(), severity: "Severe".into(), ends: Some("2026-11-16T06:00:00-07:00".into()) }
    }

    #[test]
    fn notifies_once_when_threshold_crossed() {
        let (n, s) = decide(&NotifyState::default(), &snap(Some(25.4), vec![]), 15.24, false);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].title, "10\" of snow coming to Vail");
        assert_eq!(n[0].body, "Next 72 hours. 2\" in the next 24. Biggest day: Sunday (8\").");
        assert_eq!(n[0].resort_id, "vail-co");
        assert_eq!(n[0].tag, "storm");
        let (again, s2) = decide(&s, &snap(Some(27.0), vec![]), 15.24, false);
        assert!(again.is_empty());
        assert_eq!(s2.storm_cm, Some(25.4));
    }

    #[test]
    fn below_threshold_or_unknown_is_silent() {
        assert!(decide(&NotifyState::default(), &snap(Some(15.0), vec![]), 15.24, false).0.is_empty());
        assert!(decide(&NotifyState::default(), &snap(None, vec![]), 15.24, false).0.is_empty());
    }

    #[test]
    fn renotifies_when_forecast_grows_a_lot() {
        let st = NotifyState { storm_cm: Some(25.4), alerts: vec![] };
        let (n, s) = decide(&st, &snap(Some(40.0), vec![]), 15.24, true);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].title, "Vail forecast up to 40 cm");
        assert!(n[0].body.starts_with("Was 25 cm."));
        assert_eq!(s.storm_cm, Some(40.0));
    }

    #[test]
    fn rearms_after_storm_passes_but_not_on_a_small_dip() {
        let st = NotifyState { storm_cm: Some(25.4), alerts: vec![] };
        assert_eq!(decide(&st, &snap(Some(12.0), vec![]), 15.24, false).1.storm_cm, Some(25.4));
        assert_eq!(decide(&st, &snap(Some(3.0), vec![]), 15.24, false).1.storm_cm, None);
        // Unknown forecast must not re-arm (and so re-notify) either.
        assert_eq!(decide(&st, &snap(None, vec![]), 15.24, false).1.storm_cm, Some(25.4));
    }

    #[test]
    fn new_alert_notifies_once() {
        let (n, s) = decide(&NotifyState::default(), &snap(Some(0.0), vec![warning()]), 15.24, false);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].title, "Winter Storm Warning for Vail");
        assert_eq!(n[0].body, "Heavy snow expected");
        assert_eq!(n[0].tag, "alert:Winter Storm Warning");
        assert!(decide(&s, &snap(Some(0.0), vec![warning()]), 15.24, false).0.is_empty());
        // Once it expires it is forgotten.
        assert!(decide(&s, &snap(Some(0.0), vec![]), 15.24, false).1.alerts.is_empty());
    }

    #[test]
    fn alert_fetch_failure_does_not_forget_announced_alerts() {
        let st = NotifyState { storm_cm: None, alerts: vec!["Winter Storm Warning|2026-11-16T06:00:00-07:00".into()] };
        let mut s = snap(Some(0.0), vec![]);
        s.problems.push("NWS alerts failed: timeout".into());
        assert_eq!(decide(&st, &s, 15.24, false).1.alerts.len(), 1);
    }

    #[test]
    fn stale_snapshot_never_notifies() {
        let mut s = snap(Some(50.0), vec![warning()]);
        s.stale = true;
        let (n, st) = decide(&NotifyState::default(), &s, 15.24, false);
        assert!(n.is_empty());
        assert_eq!(st, NotifyState::default());
    }
}

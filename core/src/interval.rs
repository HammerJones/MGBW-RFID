//! NWS grid data describes each value with an ISO-8601 interval such as
//! `2026-11-14T06:00:00+00:00/PT6H`. These helpers parse those and spread
//! an amount across resort-local calendar days.

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};

pub fn parse_valid_time(s: &str) -> Option<(DateTime<Utc>, Duration)> {
    let (start, dur) = s.split_once('/')?;
    let start = DateTime::parse_from_rfc3339(start).ok()?.with_timezone(&Utc);
    let dur = parse_duration(dur)?;
    if dur <= Duration::zero() {
        return None;
    }
    Some((start, dur))
}

fn parse_duration(s: &str) -> Option<Duration> {
    let rest = s.strip_prefix('P')?;
    if rest.is_empty() {
        return None;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, t),
        None => (rest, ""),
    };
    let mut total = Duration::zero();
    let mut seen = false;
    for (part, in_time) in [(date_part, false), (time_part, true)] {
        let mut num = String::new();
        for c in part.chars() {
            if c.is_ascii_digit() {
                num.push(c);
                continue;
            }
            let n: i64 = num.parse().ok()?;
            num.clear();
            seen = true;
            total += match (c, in_time) {
                ('W', false) => Duration::weeks(n),
                ('D', false) => Duration::days(n),
                ('H', true) => Duration::hours(n),
                ('M', true) => Duration::minutes(n),
                ('S', true) => Duration::seconds(n),
                _ => return None,
            };
        }
        if !num.is_empty() {
            return None;
        }
    }
    seen.then_some(total)
}

/// Split `value` across the local calendar days the interval touches, in
/// proportion to the time spent in each.
pub fn split_by_local_day<Z: TimeZone>(
    start: DateTime<Utc>,
    dur: Duration,
    value: f64,
    tz: &Z,
) -> Vec<(NaiveDate, f64)> {
    let mut out = Vec::new();
    let end = start + dur;
    let total = dur.num_seconds() as f64;
    if total <= 0.0 {
        return out;
    }
    let mut cur = start;
    while cur < end {
        let date = cur.with_timezone(tz).date_naive();
        let next_midnight = date
            .succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|ndt| tz.from_local_datetime(&ndt).earliest())
            .map(|dt| dt.with_timezone(&Utc))
            .filter(|dt| *dt > cur)
            .unwrap_or(cur + Duration::hours(24));
        let seg_end = end.min(next_midnight);
        let frac = (seg_end - cur).num_seconds() as f64 / total;
        out.push((date, value * frac));
        cur = seg_end;
    }
    out
}

/// Fraction (0..=1) of the interval that falls inside `[win_start, win_end)`.
pub fn overlap_fraction(
    start: DateTime<Utc>,
    dur: Duration,
    win_start: DateTime<Utc>,
    win_end: DateTime<Utc>,
) -> f64 {
    let end = start + dur;
    let lo = start.max(win_start);
    let hi = end.min(win_end);
    if hi <= lo || dur <= Duration::zero() {
        return 0.0;
    }
    (hi - lo).num_seconds() as f64 / dur.num_seconds() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn parses_hours() {
        let (s, d) = parse_valid_time("2026-11-14T06:00:00+00:00/PT6H").unwrap();
        assert_eq!(s, utc("2026-11-14T06:00:00Z"));
        assert_eq!(d, Duration::hours(6));
    }

    #[test]
    fn parses_days_and_mixed() {
        assert_eq!(parse_valid_time("2026-11-14T06:00:00+00:00/P1DT12H").unwrap().1, Duration::hours(36));
        assert_eq!(parse_valid_time("2026-11-14T06:00:00+00:00/P7D").unwrap().1, Duration::hours(168));
        assert_eq!(parse_valid_time("2026-11-14T06:00:00-07:00/PT30M").unwrap().1, Duration::minutes(30));
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "nope", "2026-11-14T06:00:00+00:00", "2026-11-14T06:00:00+00:00/", "2026-11-14T06:00:00+00:00/P", "2026-11-14T06:00:00+00:00/PT", "2026-11-14T06:00:00+00:00/PT6", "2026-11-14T06:00:00+00:00/P6H", "x/PT6H", "2026-11-14T06:00:00+00:00/PT0H"] {
            assert!(parse_valid_time(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn splits_across_local_midnight() {
        // 03:00–09:00 UTC is 20:00–02:00 at UTC-7: 4 h on the 13th, 2 h on the 14th.
        let tz = FixedOffset::west_opt(7 * 3600).unwrap();
        let parts = split_by_local_day(utc("2026-11-14T03:00:00Z"), Duration::hours(6), 6.0, &tz);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].0, NaiveDate::from_ymd_opt(2026, 11, 13).unwrap());
        assert!((parts[0].1 - 4.0).abs() < 1e-9);
        assert_eq!(parts[1].0, NaiveDate::from_ymd_opt(2026, 11, 14).unwrap());
        assert!((parts[1].1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn splits_multi_day_and_conserves_total() {
        let tz = chrono_tz::America::Denver;
        let parts = split_by_local_day(utc("2026-11-14T12:00:00Z"), Duration::hours(60), 30.0, &tz);
        assert_eq!(parts.len(), 3);
        let sum: f64 = parts.iter().map(|p| p.1).sum();
        assert!((sum - 30.0).abs() < 1e-9);
    }

    #[test]
    fn overlap() {
        let s = utc("2026-11-14T06:00:00Z");
        let f = overlap_fraction(s, Duration::hours(6), utc("2026-11-14T09:00:00Z"), utc("2026-11-15T09:00:00Z"));
        assert!((f - 0.5).abs() < 1e-9);
        assert_eq!(overlap_fraction(s, Duration::hours(6), utc("2026-11-15T00:00:00Z"), utc("2026-11-16T00:00:00Z")), 0.0);
    }
}

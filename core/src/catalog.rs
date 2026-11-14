//! The bundled list of US ski resorts the user can pick from.

use crate::model::Resort;
use std::sync::OnceLock;

const CSV: &str = include_str!("../resorts.csv");
const FT: f64 = 0.3048;

pub fn slug(name: &str, state: &str) -> String {
    let mut s = String::new();
    for c in format!("{name} {state}").to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() && c != '\'' && c != '.' {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

pub fn all() -> &'static [Resort] {
    static ALL: OnceLock<Vec<Resort>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v: Vec<Resort> = CSV.lines().filter_map(parse_line).collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    })
}

fn parse_line(line: &str) -> Option<Resort> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    if f.len() != 6 {
        return None;
    }
    Some(Resort {
        id: slug(f[0], f[1]),
        name: f[0].to_string(),
        state: f[1].to_string(),
        lat: f[2].parse().ok()?,
        lon: f[3].parse().ok()?,
        base_elev_m: f[4].parse::<f64>().ok().map(|x| x * FT),
        summit_elev_m: f[5].parse::<f64>().ok().map(|x| x * FT),
    })
}

/// Case-insensitive match on name or state. Name-prefix matches come first.
/// An empty query lists everything.
pub fn search(query: &str, limit: usize) -> Vec<Resort> {
    let q = query.trim().to_lowercase();
    let mut hits: Vec<(u8, &Resort)> = all()
        .iter()
        .filter_map(|r| {
            let name = r.name.to_lowercase();
            if q.is_empty() {
                Some((1, r))
            } else if name.starts_with(&q) {
                Some((0, r))
            } else if name.contains(&q) {
                Some((1, r))
            } else if r.state.to_lowercase() == q || state_name(&r.state).to_lowercase().starts_with(&q) {
                Some((2, r))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    hits.into_iter().take(limit).map(|(_, r)| r.clone()).collect()
}

pub fn state_name(code: &str) -> &'static str {
    match code {
        "AK" => "Alaska", "AZ" => "Arizona", "CA" => "California", "CO" => "Colorado",
        "ID" => "Idaho", "MA" => "Massachusetts", "ME" => "Maine", "MI" => "Michigan",
        "MN" => "Minnesota", "MT" => "Montana", "NC" => "North Carolina", "NH" => "New Hampshire",
        "NM" => "New Mexico", "NV" => "Nevada", "NY" => "New York", "OR" => "Oregon",
        "PA" => "Pennsylvania", "SD" => "South Dakota", "UT" => "Utah", "VA" => "Virginia",
        "VT" => "Vermont", "WA" => "Washington", "WI" => "Wisconsin", "WV" => "West Virginia",
        "WY" => "Wyoming",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_line_parses_with_unique_ids_inside_the_us() {
        let data_lines = CSV.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count();
        assert_eq!(all().len(), data_lines, "a catalog line failed to parse");
        assert!(all().len() >= 100);
        let mut ids = HashSet::new();
        for r in all() {
            assert!(ids.insert(&r.id), "duplicate id {}", r.id);
            assert!(crate::geo::in_us_bounds(r.lat, r.lon), "{} is outside the US", r.name);
            assert!(!state_name(&r.state).is_empty(), "unknown state {}", r.state);
            let (b, s) = (r.base_elev_m.unwrap(), r.summit_elev_m.unwrap());
            assert!(s > b && s < 4300.0 && b > 0.0, "{} elevations", r.name);
        }
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Vail", "CO"), "vail-co");
        assert_eq!(slug("Smugglers' Notch", "VT"), "smugglers-notch-vt");
        assert_eq!(slug("Mt. Hood Meadows", "OR"), "mt-hood-meadows-or");
        assert_eq!(slug("Sierra-at-Tahoe", "CA"), "sierra-at-tahoe-ca");
    }

    #[test]
    fn search_matches_name_then_state() {
        assert_eq!(search("vail", 30)[0].name, "Vail");
        assert_eq!(search("  ASPEN ", 30)[0].name, "Aspen Highlands");
        let utah = search("utah", 50);
        assert!(utah.len() >= 10 && utah.iter().all(|r| r.state == "UT"));
        assert!(search("co", 100).iter().any(|r| r.name == "Copper Mountain"));
        assert!(search("zzzz", 30).is_empty());
        assert_eq!(search("", 5).len(), 5);
        assert_eq!(search("mountain", 3).len(), 3);
    }
}

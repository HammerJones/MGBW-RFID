//! News reports about a resort, from a Google News RSS search. Snowfall
//! amounts mentioned in headlines are pulled out so the UI can show them.

use crate::http::Http;
use crate::model::{NewsItem, Resort};
use chrono::DateTime;
use quick_xml::events::Event;
use quick_xml::Reader;
use regex_lite::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

pub const MAX_ITEMS: usize = 15;

/// The name people actually use: "Vail" rather than "Vail Ski Resort".
pub fn short_name(name: &str) -> String {
    let mut s = name.trim().to_string();
    for suffix in [" Mountain Resort", " Ski Resort", " Ski Area", " Ski & Summer Resort", " Resort", " Ski Bowl"] {
        if s.len() > suffix.len() && s.to_lowercase().ends_with(&suffix.to_lowercase()) {
            s.truncate(s.len() - suffix.len());
            break;
        }
    }
    s
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn news_url(resort: &Resort) -> String {
    // The full state name finds far more than the two-letter code.
    let state = match crate::catalog::state_name(&resort.state) {
        "" => resort.state.as_str(),
        full => full,
    };
    let q = format!("\"{}\" {} snow when:7d", short_name(&resort.name), state);
    format!("https://news.google.com/rss/search?q={}&hl=en-US&gl=US&ceid=US:en", url_encode(&q))
}

/// Parse RSS 2.0 items. Malformed XML yields whatever was read before the error.
pub fn parse_rss(xml: &str) -> Vec<NewsItem> {
    let mut reader = Reader::from_str(xml);
    let mut items = Vec::new();
    let mut in_item = false;
    let mut field: Option<String> = None;
    let (mut title, mut link, mut date, mut source) = (String::new(), String::new(), String::new(), String::new());
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = AsRef::<str>::as_ref(&e.name()).to_string();
                if name == "item" {
                    in_item = true;
                    title.clear();
                    link.clear();
                    date.clear();
                    source.clear();
                } else if in_item {
                    field = Some(name);
                }
            }
            Ok(Event::Text(t)) => {
                if let (true, Some(f)) = (in_item, field.as_deref()) {
                    let raw = AsRef::<str>::as_ref(&t).to_string();
                    let text = quick_xml::escape::unescape(&raw).map(|c| c.to_string()).unwrap_or(raw);
                    push_field(f, &text, &mut title, &mut link, &mut date, &mut source);
                }
            }
            Ok(Event::GeneralRef(r)) => {
                if let (true, Some(f)) = (in_item, field.as_deref()) {
                    let name = AsRef::<str>::as_ref(&r).to_string();
                    let text = match name.as_str() {
                        "amp" => "&".to_string(),
                        "lt" => "<".to_string(),
                        "gt" => ">".to_string(),
                        "quot" => "\"".to_string(),
                        "apos" => "'".to_string(),
                        n => n
                            .strip_prefix("#x")
                            .and_then(|h| u32::from_str_radix(h, 16).ok())
                            .or_else(|| n.strip_prefix('#').and_then(|d| d.parse().ok()))
                            .and_then(char::from_u32)
                            .map(String::from)
                            .unwrap_or_default(),
                    };
                    push_field(f, &text, &mut title, &mut link, &mut date, &mut source);
                }
            }
            Ok(Event::CData(t)) => {
                if let (true, Some(f)) = (in_item, field.as_deref()) {
                    let text = AsRef::<str>::as_ref(&t).to_string();
                    push_field(f, &text, &mut title, &mut link, &mut date, &mut source);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if AsRef::<str>::as_ref(&name) == "item" {
                    in_item = false;
                    field = None;
                    if let Some(item) = build_item(&title, &link, &date, &source) {
                        items.push(item);
                    }
                } else {
                    field = None;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    items
}

fn push_field(f: &str, text: &str, title: &mut String, link: &mut String, date: &mut String, source: &mut String) {
    match f {
        "title" => title.push_str(text),
        "link" => link.push_str(text),
        "pubDate" => date.push_str(text),
        "source" => source.push_str(text),
        _ => {}
    }
}

fn build_item(title: &str, link: &str, date: &str, source: &str) -> Option<NewsItem> {
    let mut title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut source = source.trim().to_string();
    let link = link.trim();
    if title.is_empty() || !(link.starts_with("https://") || link.starts_with("http://")) {
        return None;
    }
    // Google News titles end with " - Source Name", sometimes on top of the
    // outlet's own " - Source Name" suffix.
    while let Some((head, tail)) = title.rsplit_once(" - ") {
        if head.is_empty() || !(source.is_empty() || tail.eq_ignore_ascii_case(&source)) {
            break;
        }
        if source.is_empty() {
            source = tail.to_string();
        }
        title = head.to_string();
    }
    Some(NewsItem {
        reported_snow_cm: extract_snow_cm(&title),
        published_epoch_s: DateTime::parse_from_rfc2822(date.trim()).map(|d| d.timestamp()).unwrap_or(0),
        title,
        source,
        url: link.to_string(),
    })
}

fn amount_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?i)(?:(\d+(?:\.\d+)?)\s*(?:-|–|to)\s*)?(\d+(?:\.\d+)?)\s*(?:\+|plus)?[\s-]*(inches|inch|in\.|"|”|feet|foot|ft\b|centimeters|centimetres|cm\b)"#,
        )
        .expect("valid regex")
    })
}

/// The largest snowfall amount a headline mentions, in cm. For a range
/// ("8-12 inches") the upper bound is used.
pub fn extract_snow_cm(title: &str) -> Option<f64> {
    let mut best: Option<f64> = None;
    for cap in amount_re().captures_iter(title) {
        let n: f64 = cap.get(2)?.as_str().parse().ok()?;
        let unit = cap.get(3)?.as_str().to_lowercase();
        let cm = match unit.as_str() {
            "feet" | "foot" | "ft" => n * 30.48,
            "centimeters" | "centimetres" | "cm" => n,
            _ => n * 2.54,
        };
        // Elevations and absurd values are not snowfall.
        let plausible = match unit.as_str() {
            "feet" | "foot" | "ft" => n <= 12.0,
            _ => cm <= 400.0,
        };
        if plausible && cm > 0.0 && best.map_or(true, |b| cm > b) {
            best = Some(cm);
        }
    }
    best
}

pub fn fetch_news(http: &dyn Http, resort: &Resort) -> Result<Vec<NewsItem>, String> {
    let body = http.get(&news_url(resort)).map_err(|e| format!("News failed: {e}"))?;
    Ok(select_news(parse_rss(&body), resort))
}

/// Keep items that actually name the resort, newest first, without repeats.
pub fn select_news(mut items: Vec<NewsItem>, resort: &Resort) -> Vec<NewsItem> {
    let needle = short_name(&resort.name).to_lowercase();
    let mut seen = HashSet::new();
    items.retain(|i| i.title.to_lowercase().contains(&needle) && seen.insert(i.title.to_lowercase()));
    items.sort_by(|a, b| b.published_epoch_s.cmp(&a.published_epoch_s));
    items.truncate(MAX_ITEMS);
    items
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel><title>"Vail" CO snow - Google News</title><link>https://news.google.com</link>
<item><title>Vail reports 14 inches overnight as storm lingers - Vail Daily</title><link>https://news.google.com/rss/articles/abc?oc=5&amp;x=1</link><guid isPermaLink="false">abc</guid><pubDate>Sat, 14 Nov 2026 13:00:00 GMT</pubDate><description>&lt;a href="x"&gt;Vail&lt;/a&gt;</description><source url="https://www.vaildaily.com">Vail Daily</source></item>
<item><title><![CDATA[Opening day at Vail & Beaver Creek: what to know - The Denver Post]]></title><link>https://news.google.com/rss/articles/def</link><pubDate>Fri, 13 Nov 2026 09:30:00 GMT</pubDate><source url="https://www.denverpost.com">The Denver Post</source></item>
<item><title>Colorado ski areas brace for 1-2 feet of snow - 9News</title><link>https://news.google.com/rss/articles/ghi</link><pubDate>Sat, 14 Nov 2026 15:00:00 GMT</pubDate><source url="https://www.9news.com">9News</source></item>
<item><title>Vail reports 14 inches overnight as storm lingers - Vail Daily</title><link>https://news.google.com/rss/articles/dup</link><pubDate>Sat, 14 Nov 2026 12:00:00 GMT</pubDate><source>Vail Daily</source></item>
<item><title>Vail gets first snow of the season - KDVR - KDVR</title><link>https://news.google.com/rss/articles/twice</link><pubDate>Thu, 12 Nov 2026 09:30:00 GMT</pubDate><source>KDVR</source></item>
<item><title>No link here</title></item>
<item><title>Bad scheme</title><link>javascript:alert(1)</link></item>
</channel></rss>"#;

    pub fn vail() -> Resort {
        Resort { id: "vail-co".into(), name: "Vail Ski Resort".into(), state: "CO".into(), lat: 39.6403, lon: -106.3742, base_elev_m: Some(2475.0), summit_elev_m: Some(3527.0) }
    }

    #[test]
    fn extracts_amounts() {
        let close = |a: Option<f64>, b: f64| a.map_or(false, |a| (a - b).abs() < 1e-6);
        assert!(close(extract_snow_cm("Vail reports 14 inches overnight"), 35.56));
        assert!(close(extract_snow_cm("8-12 inches expected"), 30.48));
        assert!(close(extract_snow_cm("8 to 12 inches expected"), 30.48));
        assert!(close(extract_snow_cm("2 feet of snow"), 60.96));
        assert!(close(extract_snow_cm("30 cm falls at Alta"), 30.0));
        assert!(close(extract_snow_cm("Storm drops 10+ inches"), 25.4));
        assert!(close(extract_snow_cm("A 20-inch dump, then 3 inches more"), 50.8));
        assert!(close(extract_snow_cm("Up to 18\" by Monday"), 45.72));
    }

    #[test]
    fn ignores_non_amounts() {
        assert_eq!(extract_snow_cm("Lift 14 opens"), None);
        assert_eq!(extract_snow_cm("Snow above 9,000 feet"), None);
        assert_eq!(extract_snow_cm("Resort at 11000 ft sees flakes"), None);
        assert_eq!(extract_snow_cm("Opening day is Nov. 20 in Vail"), None);
        assert_eq!(extract_snow_cm(""), None);
    }

    #[test]
    fn parses_rss_items() {
        let items = parse_rss(RSS);
        assert_eq!(items.len(), 5);
        assert_eq!(items[4].title, "Vail gets first snow of the season");
        assert_eq!(items[0].title, "Vail reports 14 inches overnight as storm lingers");
        assert_eq!(items[0].source, "Vail Daily");
        assert_eq!(items[0].url, "https://news.google.com/rss/articles/abc?oc=5&x=1");
        assert_eq!(items[0].published_epoch_s, 1794661200);
        assert!((items[0].reported_snow_cm.unwrap() - 35.56).abs() < 1e-6);
        assert_eq!(items[1].title, "Opening day at Vail & Beaver Creek: what to know");
        assert_eq!(items[1].reported_snow_cm, None);
    }

    #[test]
    fn malformed_xml_is_empty_not_a_panic() {
        assert!(parse_rss("").is_empty());
        assert!(parse_rss("<rss><channel><item><title>x</title").is_empty());
        assert!(parse_rss("<html><body>blocked</body></html>").is_empty());
        assert!(parse_rss("\u{0}\u{1}<<<>>>").is_empty());
    }

    #[test]
    fn selects_relevant_newest_first_without_duplicates() {
        let items = select_news(parse_rss(RSS), &vail());
        let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["Vail reports 14 inches overnight as storm lingers", "Opening day at Vail & Beaver Creek: what to know", "Vail gets first snow of the season"]);
    }

    #[test]
    fn url_and_short_name() {
        assert_eq!(short_name("Vail Ski Resort"), "Vail");
        assert_eq!(short_name("Copper Mountain"), "Copper Mountain");
        assert_eq!(short_name("Jackson Hole Mountain Resort"), "Jackson Hole");
        assert_eq!(short_name("Resort"), "Resort");
        assert_eq!(news_url(&vail()), "https://news.google.com/rss/search?q=%22Vail%22+Colorado+snow+when%3A7d&hl=en-US&gl=US&ceid=US:en");
    }
}

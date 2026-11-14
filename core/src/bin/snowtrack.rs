//! Desktop command-line check of the core: `snowtrack vail`.

use snowtrack_core::model::Snapshot;
use snowtrack_core::{catalog, SnowTrack};

fn inches(cm: Option<f64>) -> String {
    cm.map_or("—".to_string(), |c| format!("{:.1}\"", c / 2.54))
}

fn print(s: &Snapshot) {
    println!("{} ({})", s.resort.name, s.resort.state);
    match &s.station {
        Some(st) => println!("Snow history from SNOTEL {} ({:.0} km away)", st.name, st.distance_km),
        None => println!("No SNOTEL station nearby: forecast only"),
    }
    println!("Yesterday {}   Last 7 days {}   Depth {}", inches(s.last_24h_cm), inches(s.last_7d_cm), inches(s.depth_cm));
    println!("Next 24 h {}   Next 72 h {}   Next 7 days {}", inches(s.next_24h_cm), inches(s.next_72h_cm), inches(s.next_7d_cm));
    println!("\n{:<12}{:>8}{:>8}  {}", "Date", "Snow", "Depth", "");
    for d in &s.days {
        println!("{:<12}{:>8}{:>8}  {}", d.date, inches(d.snow_cm), inches(d.depth_cm), if d.is_forecast { "forecast" } else { "" });
    }
    for a in &s.alerts {
        println!("\n! {}", a.headline);
    }
    for n in s.news.iter().take(5) {
        println!("\n- {} ({}){}", n.title, n.source, n.reported_snow_cm.map_or(String::new(), |c| format!(" [{:.0}\" reported]", c / 2.54)));
    }
    for p in &s.problems {
        eprintln!("\nwarning: {p}");
    }
}

fn main() {
    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if query.is_empty() {
        eprintln!("usage: snowtrack <resort name>");
        std::process::exit(2);
    }
    let Some(resort) = catalog::search(&query, 1).into_iter().next() else {
        eprintln!("No resort matching \"{query}\"");
        std::process::exit(1);
    };
    let dir = std::env::temp_dir().join("snowtrack-cli");
    let app = SnowTrack::new(dir.to_string_lossy().to_string());
    let _ = app.add_resort(resort.clone());
    match app.refresh(resort.id) {
        Ok(s) => print(&s),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

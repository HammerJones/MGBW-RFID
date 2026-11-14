//! In-app updates: every build is published as a GitHub release with the
//! APK attached, tagged `build-<number>`.

use crate::http::Http;
use crate::model::UpdateInfo;
use serde_json::Value;
use std::path::Path;

/// `owner/repo`. Set by GitHub Actions at build time so a renamed repo keeps working.
pub fn repo() -> &'static str {
    option_env!("GITHUB_REPOSITORY").filter(|r| r.contains('/')).unwrap_or("HammerJones/MGBW-RFID")
}

pub fn latest_url() -> String {
    format!("https://api.github.com/repos/{}/releases/latest", repo())
}

/// The newest release, if it is newer than the running build.
pub fn check(http: &dyn Http, current_build: u32) -> Result<Option<UpdateInfo>, String> {
    let body = http.get(&latest_url()).map_err(|e| format!("Couldn't check for updates: {e}"))?;
    parse_latest(&body, current_build)
}

pub fn parse_latest(body: &str, current_build: u32) -> Result<Option<UpdateInfo>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("Couldn't check for updates: unreadable response ({e})"))?;
    let tag = v["tag_name"].as_str().ok_or("Couldn't check for updates: no release found")?;
    let Some(build) = tag.strip_prefix("build-").and_then(|n| n.parse::<u32>().ok()) else {
        return Ok(None);
    };
    if build <= current_build {
        return Ok(None);
    }
    let apk_url = v["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["name"].as_str().is_some_and(|n| n.ends_with(".apk")))
        .find_map(|a| a["browser_download_url"].as_str())
        .filter(|u| is_trusted_url(u));
    let Some(apk_url) = apk_url else { return Ok(None) };
    Ok(Some(UpdateInfo {
        version: v["name"].as_str().filter(|n| !n.is_empty()).unwrap_or(tag).to_string(),
        build,
        apk_url: apk_url.to_string(),
        notes: v["body"].as_str().unwrap_or("").trim().chars().take(400).collect(),
    }))
}

/// Only this project's own release downloads are ever fetched and installed.
pub fn is_trusted_url(url: &str) -> bool {
    let prefix = format!("https://github.com/{}/releases/download/", repo()).to_lowercase();
    url.to_lowercase().starts_with(&prefix) && !url.contains("..") && !url.contains('?') && !url.contains('#')
}

pub fn download(http: &dyn Http, url: &str, dest: &Path) -> Result<(), String> {
    if !is_trusted_url(url) {
        return Err("That update link isn't from the SnowTrack releases page".to_string());
    }
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let part = dest.with_extension("part");
    let result = http.download(url, &part).map_err(|e| format!("Update download failed: {e}")).and_then(|_| {
        // An APK is a zip file; anything else (an error page, a cut-off download) must not reach the installer.
        let mut magic = [0u8; 4];
        let ok = std::fs::File::open(&part).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut magic)).is_ok() && &magic == b"PK\x03\x04";
        if ok {
            std::fs::rename(&part, dest).map_err(|e| format!("Couldn't save the update: {e}"))
        } else {
            Err("The downloaded update wasn't a valid app file".to_string())
        }
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::fake::FakeHttp;
    use serde_json::json;

    fn release(tag: &str, url: &str) -> String {
        json!({"tag_name": tag, "name": "SnowTrack 0.1.42", "body": "Scrub bar for the radar\n",
               "assets": [{"name": "notes.txt", "browser_download_url": "https://github.com/HammerJones/MGBW-RFID/releases/download/build-42/notes.txt"},
                          {"name": "SnowTrack.apk", "browser_download_url": url}]})
        .to_string()
    }

    const GOOD: &str = "https://github.com/HammerJones/MGBW-RFID/releases/download/build-42/SnowTrack.apk";

    #[test]
    fn newer_release_is_offered() {
        let u = parse_latest(&release("build-42", GOOD), 41).unwrap().unwrap();
        assert_eq!(u, UpdateInfo { version: "SnowTrack 0.1.42".into(), build: 42, apk_url: GOOD.into(), notes: "Scrub bar for the radar".into() });
    }

    #[test]
    fn same_older_or_odd_releases_are_not() {
        assert_eq!(parse_latest(&release("build-42", GOOD), 42).unwrap(), None);
        assert_eq!(parse_latest(&release("build-42", GOOD), 50).unwrap(), None);
        assert_eq!(parse_latest(&release("v1.0", GOOD), 1).unwrap(), None);
        assert_eq!(parse_latest(&json!({"tag_name": "build-42", "assets": []}).to_string(), 1).unwrap(), None);
        assert!(parse_latest(&json!({"message": "Not Found"}).to_string(), 1).is_err());
        assert!(parse_latest("<html>", 1).is_err());
    }

    #[test]
    fn only_this_repos_release_downloads_are_trusted() {
        assert!(is_trusted_url(GOOD));
        assert!(is_trusted_url("https://github.com/hammerjones/mgbw-rfid/releases/download/build-9/SnowTrack.apk"));
        for bad in [
            "http://github.com/HammerJones/MGBW-RFID/releases/download/build-42/SnowTrack.apk",
            "https://github.com/someone-else/MGBW-RFID/releases/download/build-42/SnowTrack.apk",
            "https://github.com.evil.example/HammerJones/MGBW-RFID/releases/download/x.apk",
            "https://evil.example/https://github.com/HammerJones/MGBW-RFID/releases/download/x.apk",
            "https://github.com/HammerJones/MGBW-RFID/releases/download/../../../evil/x.apk",
            "",
        ] {
            assert!(!is_trusted_url(bad), "{bad}");
        }
        // A release whose asset points elsewhere is ignored rather than offered.
        assert_eq!(parse_latest(&release("build-42", "https://evil.example/SnowTrack.apk"), 1).unwrap(), None);
    }

    #[test]
    fn download_keeps_only_real_apks() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("updates").join("SnowTrack.apk");
        let good = FakeHttp::new().ok("build-42/SnowTrack.apk", "PK\u{3}\u{4}rest-of-the-apk");
        download(&good, GOOD, &dest).unwrap();
        assert!(std::fs::read(&dest).unwrap().starts_with(b"PK\x03\x04"));

        let dest2 = dir.path().join("bad.apk");
        let html = FakeHttp::new().ok("build-42/SnowTrack.apk", "<html>Not Found</html>");
        assert!(download(&html, GOOD, &dest2).is_err());
        assert!(!dest2.exists() && !dest2.with_extension("part").exists());
        assert!(download(&FakeHttp::new(), GOOD, &dest2).is_err());
        assert!(download(&good, "https://evil.example/x.apk", &dest2).is_err());
        assert_eq!(good.count("evil"), 0);
    }
}

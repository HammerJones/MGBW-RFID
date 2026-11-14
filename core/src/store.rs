//! On-disk state: the user's resorts, settings and cached data, as JSON
//! files in the app's private directory.

use crate::model::{Resort, Settings, Snapshot, StationInfo};
use crate::notify::NotifyState;
use crate::nws::PointInfo;
use crate::snotel::Station;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tracked {
    pub resort: Resort,
    #[serde(default)]
    pub notify: bool,
    #[serde(default)]
    pub notify_state: NotifyState,
    /// Cached NWS grid lookup for this location.
    #[serde(default)]
    pub point: Option<PointInfo>,
    /// True once we have looked for a nearby SNOTEL station; `station`
    /// stays `None` when there is none within range.
    #[serde(default)]
    pub station_checked: bool,
    #[serde(default)]
    pub station: Option<StationInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub tracked: Vec<Tracked>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationCache {
    pub fetched_at_epoch_s: i64,
    pub stations: Vec<Station>,
}

pub struct Store {
    dir: PathBuf,
}

fn safe_id(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

impl Store {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        Store { dir }
    }

    fn read<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let text = fs::read_to_string(self.dir.join(name)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Write via a temporary file so a crash never leaves a half-written file.
    fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let path = self.dir.join(name);
        // Unique per write: the UI and the background check can save the same file at once.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = self.dir.join(format!("{name}.{}.{n}.tmp", std::process::id()));
        let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).and_then(|_| fs::rename(&tmp, &path)).map_err(|e| format!("Could not save {name}: {e}"))
    }

    pub fn load_state(&self) -> State {
        let path = self.dir.join("state.json");
        match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(s) => s,
                Err(_) => {
                    // Keep the unreadable file for recovery rather than overwriting it.
                    let _ = fs::rename(&path, self.dir.join("state.corrupt.json"));
                    State::default()
                }
            },
            Err(_) => State::default(),
        }
    }

    pub fn save_state(&self, state: &State) -> Result<(), String> {
        self.write("state.json", state)
    }

    pub fn load_snapshot(&self, id: &str) -> Option<Snapshot> {
        self.read(&format!("snap_{}.json", safe_id(id)))
    }

    pub fn save_snapshot(&self, snap: &Snapshot) -> Result<(), String> {
        self.write(&format!("snap_{}.json", safe_id(&snap.resort.id)), snap)
    }

    pub fn delete_snapshot(&self, id: &str) {
        let _ = fs::remove_file(self.dir.join(format!("snap_{}.json", safe_id(id))));
    }

    pub fn load_stations(&self) -> Option<StationCache> {
        self.read("stations.json")
    }

    pub fn save_stations(&self, cache: &StationCache) -> Result<(), String> {
        self.write("stations.json", cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked() -> Tracked {
        Tracked { resort: crate::news::tests::vail(), notify: true, notify_state: NotifyState::default(), point: None, station_checked: true, station: None }
    }

    #[test]
    fn state_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("nested"));
        assert_eq!(store.load_state(), State::default());
        let state = State { tracked: vec![tracked()], settings: Settings { metric: true, notify_threshold_cm: 20.0 } };
        store.save_state(&state).unwrap();
        assert_eq!(Store::new(dir.path().join("nested")).load_state(), state);
    }

    #[test]
    fn corrupt_state_is_set_aside_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("state.json"), "{ not json").unwrap();
        let store = Store::new(dir.path());
        assert_eq!(store.load_state(), State::default());
        assert_eq!(fs::read_to_string(dir.path().join("state.corrupt.json")).unwrap(), "{ not json");
    }

    #[test]
    fn older_state_without_new_fields_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let resort = serde_json::to_string(&crate::news::tests::vail()).unwrap();
        fs::write(dir.path().join("state.json"), format!("{{\"tracked\":[{{\"resort\":{resort}}}]}}")).unwrap();
        let s = Store::new(dir.path()).load_state();
        assert_eq!(s.tracked.len(), 1);
        assert!(!s.tracked[0].notify);
        assert_eq!(s.settings, Settings::default());
    }

    #[test]
    fn snapshot_ids_cannot_escape_the_directory() {
        assert_eq!(safe_id("../../etc/passwd"), "______etc_passwd");
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        assert!(store.load_snapshot("../x").is_none());
        store.delete_snapshot("nope");
    }
}

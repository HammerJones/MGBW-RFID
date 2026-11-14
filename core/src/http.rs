//! All network access goes through `Http` so parsers and the refresh flow
//! can be tested without touching the network.

use std::time::Duration;

pub const USER_AGENT: &str = "SnowTrack/0.1 (github.com/HammerJones)";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct HttpError(pub String);

pub trait Http: Send + Sync {
    fn get(&self, url: &str) -> Result<String, HttpError>;
}

pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .user_agent(USER_AGENT)
            .build()
            .into();
        UreqHttp { agent }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl Http for UreqHttp {
    fn get(&self, url: &str) -> Result<String, HttpError> {
        let mut resp = self
            .agent
            .get(url)
            .header("Accept", "application/geo+json, application/json, application/xml, */*")
            .call()
            .map_err(|e| HttpError(e.to_string()))?;
        resp.body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| HttpError(e.to_string()))
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::Mutex;

    /// Answers a request with the first rule whose pattern is a substring of the URL.
    #[derive(Default)]
    pub struct FakeHttp {
        rules: Mutex<Vec<(String, Result<String, HttpError>)>>,
        pub log: Mutex<Vec<String>>,
    }

    impl FakeHttp {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn ok(self, pattern: &str, body: impl Into<String>) -> Self {
            self.rules.lock().unwrap().push((pattern.into(), Ok(body.into())));
            self
        }
        pub fn fail(self, pattern: &str, msg: &str) -> Self {
            self.rules.lock().unwrap().push((pattern.into(), Err(HttpError(msg.into()))));
            self
        }
        pub fn count(&self, pattern: &str) -> usize {
            self.log.lock().unwrap().iter().filter(|u| u.contains(pattern)).count()
        }
    }

    impl Http for FakeHttp {
        fn get(&self, url: &str) -> Result<String, HttpError> {
            self.log.lock().unwrap().push(url.to_string());
            for (pat, res) in self.rules.lock().unwrap().iter() {
                if url.contains(pat.as_str()) {
                    return res.clone();
                }
            }
            Err(HttpError(format!("no fake response for {url}")))
        }
    }

    #[test]
    fn fake_http_matches_by_substring() {
        let h = FakeHttp::new().ok("/points/", "P").ok("alerts", "A");
        assert_eq!(h.get("https://api.weather.gov/points/1,2").unwrap(), "P");
        assert_eq!(h.get("https://x/alerts/active").unwrap(), "A");
        assert_eq!(h.count("points"), 1);
    }

    #[test]
    fn fake_http_unmatched_is_error() {
        let h = FakeHttp::new().fail("bad", "boom");
        assert!(h.get("https://nowhere").is_err());
        assert_eq!(h.get("https://bad").unwrap_err().0, "boom");
    }
}

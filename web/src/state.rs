use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use analytics::connector::FixtureConnector;
use analytics::pipeline::{analyze_with_packs, Analysis};

use crate::error::WebError;
use crate::overlay::Overlay;

pub struct AppState {
    pub fixtures_root: PathBuf,
    pub overlay_root: PathBuf,
    cache: Mutex<HashMap<String, Analysis>>,
}

impl AppState {
    pub fn new(fixtures_root: PathBuf, overlay_root: PathBuf) -> Self {
        Self {
            fixtures_root,
            overlay_root,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn list_fixtures(&self) -> Vec<String> {
        list_fixtures(&self.fixtures_root)
    }

    pub fn fixture_dir(&self, name: &str) -> Result<PathBuf, WebError> {
        if !fixture_id_ok(name) {
            return Err(WebError::bad(format!("invalid fixture name `{name}`")));
        }
        let path = self.fixtures_root.join(name);
        if !path.is_dir() {
            return Err(WebError::not_found(format!("fixture `{name}` not found")));
        }
        Ok(path)
    }

    pub fn analyze(&self, name: &str, packs: &str) -> Result<Analysis, WebError> {
        let key = format!("{name}\n{packs}");
        if let Some(hit) = self.cache.lock().expect("cache").get(&key).cloned() {
            return Ok(hit);
        }
        let path = self.fixture_dir(name)?;
        let connector = FixtureConnector::open(&path).map_err(WebError::from_analytics)?;
        let analysis = analyze_with_packs(&connector, packs).map_err(WebError::from_analytics)?;
        self.cache
            .lock()
            .expect("cache")
            .insert(key, analysis.clone());
        Ok(analysis)
    }

    pub fn connector(&self, name: &str) -> Result<FixtureConnector, WebError> {
        let path = self.fixture_dir(name)?;
        FixtureConnector::open(path).map_err(WebError::from_analytics)
    }

    pub fn load_overlay(&self, name: &str, packs: &[String]) -> Overlay {
        Overlay::load(&self.overlay_root, name, packs)
    }

    pub fn save_overlay(&self, overlay: &Overlay) -> Result<(), WebError> {
        overlay
            .save(&self.overlay_root)
            .map_err(WebError::from_analytics)
    }
}

pub fn fixture_id_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn list_fixtures(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return names;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !fixture_id_ok(&name) {
            continue;
        }
        let has_csv = std::fs::read_dir(&path)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|f| f.path().extension().and_then(|s| s.to_str()) == Some("csv"));
        if has_csv {
            names.push(name);
        }
    }
    names.sort();
    names
}

pub fn parse_packs(raw: &str) -> String {
    let parts: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();
    if parts.is_empty() {
        analytics::dictionary::DEFAULT_PACKS.to_string()
    } else {
        parts.join(",")
    }
}

pub fn pack_list(packs: &str) -> Vec<String> {
    packs
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_escape() {
        assert!(!fixture_id_ok("../etc"));
        assert!(!fixture_id_ok("foo/bar"));
        assert!(!fixture_id_ok(""));
        assert!(fixture_id_ok("ecommerce_clean"));
        assert!(fixture_id_ok("edge_cases"));
    }
}

//! The manual-rename opt-out store.
//!
//! Herdr has no per-tab metadata, so which tabs Heraldr owns is tracked here:
//! per `tab_id`, the last base label Heraldr set and whether auto-naming is
//! still enabled. Each session's watcher and one-shot commands share a
//! socket-keyed store; other sessions cannot overwrite or prune its tabs.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::naming::is_placeholder;
use crate::rpc::{Client, home};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TabState {
    pub auto: String,
    pub enabled: bool,
}

#[derive(Debug, Default)]
pub struct Store {
    path: PathBuf,
    tabs: HashMap<String, TabState>,
    dirty: bool,
}

/// `HERALDR_STATE_DIR` overrides the store location (tests).
pub fn state_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("HERALDR_STATE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        return PathBuf::from(dir);
    }
    standalone_state_dir()
}

fn standalone_state_dir() -> PathBuf {
    let state_home = std::env::var_os("XDG_STATE_HOME")
        .map_or_else(|| home().join(".local/state"), PathBuf::from);
    state_home.join("herdr-heraldr")
}

pub fn session_key(client: &Client) -> u64 {
    let mut hasher = DefaultHasher::new();
    client.socket_path().hash(&mut hasher);
    hasher.finish()
}

impl Store {
    /// Absent session files inherit shared-format ownership without changing it.
    pub fn load(client: &Client) -> Self {
        let path = state_dir().join(format!("state-{:016x}.json", session_key(client)));
        let (text, initialize) = match std::fs::read_to_string(&path) {
            Ok(text) => (Some(text), false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                std::fs::read_to_string(state_dir().join("state.json"))
                    .ok()
                    .or_else(|| {
                        if std::env::var_os("HERALDR_STATE_DIR").is_some()
                            || std::env::var_os("HERDR_PLUGIN_STATE_DIR").is_none()
                        {
                            return None;
                        }
                        let standalone = standalone_state_dir();
                        std::fs::read_to_string(
                            standalone.join(format!("state-{:016x}.json", session_key(client))),
                        )
                        .or_else(|_| std::fs::read_to_string(standalone.join("state.json")))
                        .ok()
                    }),
                true,
            ),
            Err(_) => (None, false),
        };
        let tabs = text
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            path,
            tabs,
            dirty: initialize,
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.tabs)?)?;
        std::fs::rename(&tmp, &self.path)?;
        self.dirty = false;
        Ok(())
    }

    /// Record that Heraldr owns a tab and last set this base label.
    pub fn record(&mut self, tab: &str, auto: &str) {
        let entry = self.tabs.entry(tab.to_string()).or_default();
        if !entry.enabled || entry.auto != auto {
            entry.auto = auto.to_string();
            entry.enabled = true;
            self.dirty = true;
        }
    }

    fn opt_out(&mut self, tab: &str) {
        let entry = self.tabs.entry(tab.to_string()).or_default();
        entry.auto.clear();
        entry.enabled = false;
        self.dirty = true;
    }

    /// Drop a tab's entry entirely so the next pass re-adopts it (reset).
    pub fn forget(&mut self, tab: &str) {
        if self.tabs.remove(tab).is_some() {
            self.dirty = true;
        }
    }

    /// Drop entries for tabs that no longer exist.
    pub fn prune(&mut self, keep: &HashSet<String>) {
        let before = self.tabs.len();
        self.tabs.retain(|tab, _| keep.contains(tab));
        if self.tabs.len() != before {
            self.dirty = true;
        }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// The manual-rename opt-out state machine. `base` is the tab's current
    /// label with any "[N] " prefix already stripped. Returns whether the tab
    /// is eligible for auto-naming; may record an opt-out as a side effect.
    pub fn eligible(&mut self, tab: &str, base: &str) -> bool {
        match self.tabs.get(tab) {
            // First sighting: adopt herdr's generated placeholder label;
            // anything else was named by hand.
            None => {
                if is_placeholder(base) {
                    true
                } else {
                    self.opt_out(tab);
                    false
                }
            }
            // Opted out. Re-adopt only on an explicit clear (empty label); a
            // numeric label is a deliberate name (use reset instead).
            Some(state) if !state.enabled => base.is_empty(),
            // Owned: keep updating while the base still matches what Heraldr
            // last set; an empty label is the user clearing it back to us.
            Some(state) => {
                if base == state.auto || base.is_empty() {
                    true
                } else {
                    self.opt_out(tab);
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::default()
    }

    #[test]
    fn first_sighting_adopts_placeholders_only() {
        let mut s = store();
        assert!(s.eligible("t1", "3"), "integer placeholder adopts");
        assert!(s.eligible("t2", ""), "empty label adopts");
        assert!(!s.eligible("t3", "notes"), "hand-named tab opts out");
        assert!(!s.eligible("t3", "notes"), "and stays opted out");
    }

    #[test]
    fn owned_tab_follows_heraldr_until_renamed() {
        let mut s = store();
        s.record("t1", "auto-label");
        assert!(s.eligible("t1", "auto-label"), "unchanged base stays owned");
        assert!(s.eligible("t1", ""), "cleared label re-adopts");
        assert!(!s.eligible("t1", "my notes"), "manual rename opts out");
        assert!(
            !s.eligible("t1", "42"),
            "numeric label after opt-out is deliberate"
        );
        assert!(
            s.eligible("t1", ""),
            "explicit clear re-adopts after opt-out"
        );
    }

    #[test]
    fn forget_makes_a_tab_first_seen_again() {
        let mut s = store();
        assert!(!s.eligible("t1", "notes"));
        s.forget("t1");
        assert!(s.eligible("t1", "3"), "reset re-adopts a placeholder label");
    }

    #[test]
    fn prune_drops_closed_tabs() {
        let mut s = store();
        s.record("t1", "a");
        s.record("t2", "b");
        s.prune(&HashSet::from(["t1".to_string()]));
        assert_eq!(s.len(), 1);
    }
}

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentIcons {
    #[default]
    Auto,
    Font,
    None,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub agent_icons: AgentIcons,
    pub agent_session_titles: bool,
    pub session_title_max_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_icons: AgentIcons::Auto,
            agent_session_titles: false,
            session_title_max_width: 28,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(&path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error).with_context(|| format!("reading {}", path.display()));
            }
        };
        let config: Self = serde_json::from_str(&text)
            .with_context(|| format!("invalid JSON settings in {}", path.display()))?;
        ensure!(
            (1..=256).contains(&config.session_title_max_width),
            "{}: session_title_max_width must be between 1 and 256",
            path.display()
        );
        Ok(config)
    }
}

pub fn path() -> PathBuf {
    config_path(
        std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
    )
}

fn config_path(plugin: Option<PathBuf>, xdg: Option<PathBuf>, home: Option<&Path>) -> PathBuf {
    plugin
        .filter(|dir| !dir.as_os_str().is_empty())
        .map_or_else(
            || {
                xdg.filter(|dir| !dir.as_os_str().is_empty())
                    .unwrap_or_else(|| home.unwrap_or_else(|| Path::new(".")).join(".config"))
                    .join("herdr/plugins/config/heraldr/config.json")
            },
            |dir| dir.join("config.json"),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn file(contents: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "heraldr-config-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn omitted_settings_preserve_existing_tab_titles() {
        let path = file("{}");
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.agent_icons, AgentIcons::Auto);
        assert!(!config.agent_session_titles);
        assert_eq!(config.session_title_max_width, 28);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reads_agent_display_preferences() {
        let path = file(
            r#"{"agent_icons":"font","agent_session_titles":true,"session_title_max_width":40}"#,
        );
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.agent_icons, AgentIcons::Font);
        assert!(config.agent_session_titles);
        assert_eq!(config.session_title_max_width, 40);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_settings_identify_the_config_file() {
        for contents in [
            "{",
            r#"{"agent_icons":"yes"}"#,
            r#"{"agent_session_title":true}"#,
            r#"{"session_title_max_width":0}"#,
            r#"{"session_title_max_width":257}"#,
        ] {
            let path = file(contents);
            let error = Config::load_from(&path).unwrap_err().to_string();
            assert!(error.contains(path.to_str().unwrap()), "{error}");
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn config_locations_follow_plugin_then_xdg_then_home() {
        let home = Some(Path::new("/home/winnie"));
        assert_eq!(
            config_path(Some("/plugin".into()), Some("/xdg".into()), home),
            Path::new("/plugin/config.json")
        );
        assert_eq!(
            config_path(None, Some("/xdg".into()), home),
            Path::new("/xdg/herdr/plugins/config/heraldr/config.json")
        );
        assert_eq!(
            config_path(None, None, home),
            Path::new("/home/winnie/.config/herdr/plugins/config/heraldr/config.json")
        );
    }

    #[test]
    fn missing_config_uses_defaults() {
        let path = file("");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap(), Config::default());
    }
}

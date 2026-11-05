use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentIcons {
    #[default]
    Auto,
    Font,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum TabToken {
    #[serde(rename = "$jump")]
    Jump,
    #[serde(rename = "$icon")]
    Icon,
    #[serde(rename = "$program")]
    Program,
    #[serde(rename = "$session_title")]
    SessionTitle,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub agent_icons: AgentIcons,
    pub tab_label: Vec<TabToken>,
    pub session_title_max_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_icons: AgentIcons::Auto,
            tab_label: vec![TabToken::Jump, TabToken::Icon, TabToken::Program],
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
        ensure!(
            (1..=4).contains(&config.tab_label.len()),
            "{}: tab_label must contain between 1 and 4 tokens",
            path.display()
        );
        ensure!(
            config
                .tab_label
                .iter()
                .enumerate()
                .all(|(index, token)| !config.tab_label[..index].contains(token)),
            "{}: tab_label tokens must be unique",
            path.display()
        );
        Ok(config)
    }
}

pub fn path() -> PathBuf {
    directory().join("config.json")
}

pub fn directory() -> PathBuf {
    config_directory(
        std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
    )
}

fn config_directory(plugin: Option<PathBuf>, xdg: Option<PathBuf>, home: Option<&Path>) -> PathBuf {
    plugin
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| {
            xdg.filter(|dir| !dir.as_os_str().is_empty())
                .unwrap_or_else(|| home.unwrap_or_else(|| Path::new(".")).join(".config"))
                .join("herdr/plugins/config/heraldr")
        })
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
        assert_eq!(
            config.tab_label,
            vec![TabToken::Jump, TabToken::Icon, TabToken::Program]
        );
        assert_eq!(config.session_title_max_width, 28);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reads_agent_display_preferences() {
        let path = file(
            r#"{"agent_icons":"font","tab_label":["$jump","$icon","$program","$session_title"],"session_title_max_width":40}"#,
        );
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.agent_icons, AgentIcons::Font);
        assert_eq!(
            config.tab_label,
            vec![
                TabToken::Jump,
                TabToken::Icon,
                TabToken::Program,
                TabToken::SessionTitle
            ]
        );
        assert_eq!(config.session_title_max_width, 40);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tab_tokens_can_be_selected_and_reordered() {
        let path = file(r#"{"tab_label":["$session_title","$jump"]}"#);
        assert_eq!(
            Config::load_from(&path).unwrap().tab_label,
            vec![TabToken::SessionTitle, TabToken::Jump]
        );
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
    fn tab_labels_require_unique_known_tokens() {
        for contents in [
            r#"{"tab_label":[]}"#,
            r#"{"tab_label":["$icon","$icon"]}"#,
            r#"{"tab_label":["$jump","$icon","$program","$session_title","$jump"]}"#,
            r#"{"tab_label":["$task"]}"#,
            r#"{"tab_label":["jump"]}"#,
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
            config_directory(Some("/plugin".into()), Some("/xdg".into()), home),
            Path::new("/plugin")
        );
        assert_eq!(
            config_directory(None, Some("/xdg".into()), home),
            Path::new("/xdg/herdr/plugins/config/heraldr")
        );
        assert_eq!(
            config_directory(None, None, home),
            Path::new("/home/winnie/.config/herdr/plugins/config/heraldr")
        );
    }

    #[test]
    fn missing_config_uses_defaults() {
        let path = file("");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap(), Config::default());
    }
}

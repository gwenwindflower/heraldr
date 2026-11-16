use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

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
        let config: Self = if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            toml::from_str(&text)
                .with_context(|| format!("invalid TOML settings in {}", path.display()))?
        } else {
            serde_json::from_str(&text)
                .with_context(|| format!("invalid JSON settings in {}", path.display()))?
        };
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
    path_in(&directory())
}

fn path_in(directory: &Path) -> PathBuf {
    let json = directory.join("config.json");
    if json.symlink_metadata().is_ok() {
        json
    } else {
        directory.join("config.toml")
    }
}

pub fn initialize() -> Result<(PathBuf, bool)> {
    let directory = installation_directory()?;
    let selected = path_in(&directory);
    if selected.symlink_metadata().is_ok() {
        return Ok((selected, false));
    }
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    let path = directory.join("config.toml");
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Ok((path, false));
        }
        Err(error) => return Err(error).with_context(|| format!("creating {}", path.display())),
    };
    file.write_all(DEFAULT_CONFIG.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok((path, true))
}

fn installation_directory() -> Result<PathBuf> {
    if std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").is_some_and(|value| !value.is_empty()) {
        return Ok(directory());
    }
    let output = match std::process::Command::new("herdr")
        .args(["plugin", "config-dir", "heraldr"])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(directory()),
        Err(error) => return Err(error).context("locating Heraldr configuration with Herdr"),
    };
    ensure!(
        output.status.success(),
        "herdr plugin config-dir heraldr failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let path =
        String::from_utf8(output.stdout).context("reading Herdr's plugin config directory")?;
    ensure!(
        !path.trim().is_empty(),
        "Herdr returned an empty plugin config directory"
    );
    Ok(PathBuf::from(path.trim()))
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
    fn installed_template_explicitly_sets_every_runtime_default() {
        let installed: serde_json::Value = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(installed, serde_json::to_value(Config::default()).unwrap());
    }

    #[test]
    fn existing_json_preferences_take_precedence_over_toml() {
        let root = file("");
        std::fs::remove_file(&root).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("config.toml"), "agent_icons = \"none\"\n").unwrap();
        assert_eq!(
            Config::load_from(&path_in(&root)).unwrap().agent_icons,
            AgentIcons::None
        );
        std::fs::write(root.join("config.json"), r#"{"agent_icons":"font"}"#).unwrap();
        assert_eq!(
            Config::load_from(&path_in(&root)).unwrap().agent_icons,
            AgentIcons::Font
        );
        std::fs::remove_dir_all(root).unwrap();
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
    fn commented_toml_accepts_the_same_label_preferences() {
        let original = file(
            "# Compose the tab\ntab_label = [\"$jump\", \"$session_title\"]\nagent_icons = \"none\"\n",
        );
        let path = original.with_extension("toml");
        std::fs::rename(original, &path).unwrap();
        let config = Config::load_from(&path).unwrap();
        assert_eq!(
            config.tab_label,
            vec![TabToken::Jump, TabToken::SessionTitle]
        );
        assert_eq!(config.agent_icons, AgentIcons::None);
        assert_eq!(config.session_title_max_width, 28);
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

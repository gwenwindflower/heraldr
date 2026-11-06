use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::AgentIcons;

const FONT_RECHECK: Duration = Duration::from_secs(60);
static FONT: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

// Herdr Agent Icons Max: https://github.com/hhdebb/herdr-radar/blob/main/tools/codepoints.toml
const VENDORS: &[(&str, &str)] = &[
    ("claude", "\u{e1a0}"),
    ("codex", "\u{e1a1}"),
    ("opencode", "\u{e1a2}"),
    ("omp", "\u{e1a3}"),
    ("cline", "\u{e1a4}"),
    ("mastracode", "\u{e1a5}"),
    ("kimi", "\u{e1a6}"),
    ("kilo", "\u{e1a7}"),
    ("maki", "\u{e1a8}"),
    ("pi", "\u{e1a9}"),
    ("hermes", "\u{e1aa}"),
    ("cursor", "\u{e1ab}"),
    ("copilot", "\u{e1ac}"),
    ("deepseek", "\u{e1ad}"),
    ("gemini", "\u{e1ae}"),
    ("gpt", "\u{e1af}"),
    ("qwen", "\u{e1b0}"),
    ("grok", "\u{e1b1}"),
    ("agy", "\u{e1b2}"),
    ("kiro", "\u{e1b3}"),
    ("amp", "\u{e1b4}"),
    ("devin", "\u{e1b5}"),
    ("qodercli", "\u{e1b6}"),
    ("glm", "\u{e1b7}"),
];

pub fn canonical_agent(program: &str) -> Option<&'static str> {
    let program = match program {
        "claude-code" | "claude code" => "claude",
        "cursor-agent" => "cursor",
        "open-code" => "opencode",
        other => other,
    };
    VENDORS
        .iter()
        .find(|(vendor, _)| *vendor == program)
        .map(|(vendor, _)| *vendor)
}

pub fn glyph(program: &str, mode: AgentIcons) -> Option<&'static str> {
    let agent = canonical_agent(program)?;
    let available = match mode {
        AgentIcons::Auto => font_available(),
        AgentIcons::Font => true,
        AgentIcons::None => false,
    };
    glyph_for(agent, available)
}

fn glyph_for(agent: &str, available: bool) -> Option<&'static str> {
    available
        .then(|| {
            VENDORS
                .iter()
                .find(|(vendor, _)| *vendor == agent)
                .map(|(_, glyph)| *glyph)
        })
        .flatten()
}

pub fn font_available() -> bool {
    let mut cache = FONT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((checked, available)) = *cache
        && checked.elapsed() < FONT_RECHECK
    {
        return available;
    }
    let available = font_directories().iter().any(|dir| has_font_file(dir, 3));
    *cache = Some((Instant::now(), available));
    available
}

fn font_directories() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from);
    if cfg!(target_os = "macos") {
        let mut directories = vec![
            PathBuf::from("/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts"),
        ];
        if let Some(home) = home {
            directories.push(home.join("Library/Fonts"));
        }
        directories
    } else {
        let mut directories = vec![
            PathBuf::from("/usr/local/share/fonts"),
            PathBuf::from("/usr/share/fonts"),
        ];
        if let Some(data) = std::env::var_os("XDG_DATA_HOME").filter(|data| !data.is_empty()) {
            directories.push(PathBuf::from(data).join("fonts"));
        } else if let Some(home) = &home {
            directories.push(home.join(".local/share/fonts"));
        }
        if let Some(home) = home {
            directories.push(home.join(".fonts"));
        }
        if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
            directories.extend(
                std::env::split_paths(&data_dirs)
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .map(|dir| dir.join("fonts")),
            );
        }
        directories
    }
}

fn has_font_file(directory: &Path, depth: usize) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let Ok(kind) = entry.file_type() else {
            return false;
        };
        if kind.is_dir() {
            return depth > 0 && has_font_file(&entry.path(), depth - 1);
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        kind.is_file() && name.starts_with("HerdrAgentIconsMax") && name.ends_with(".ttf")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_mode_uses_the_published_vendor_codepoints() {
        assert_eq!(glyph("claude", AgentIcons::Font), Some("\u{e1a0}"));
        assert_eq!(glyph("codex", AgentIcons::Font), Some("\u{e1a1}"));
        assert_eq!(glyph("pi", AgentIcons::Font), Some("\u{e1a9}"));
        assert_eq!(glyph("glm", AgentIcons::Font), Some("\u{e1b7}"));
    }

    #[test]
    fn none_mode_and_unknown_agents_keep_the_program_icon() {
        assert_eq!(glyph("claude", AgentIcons::None), None);
        assert_eq!(glyph("fish", AgentIcons::Font), None);
        assert_eq!(glyph("future-agent", AgentIcons::Font), None);
    }

    #[test]
    fn command_aliases_share_vendor_icons() {
        assert_eq!(
            glyph("claude-code", AgentIcons::Font),
            glyph("claude", AgentIcons::Font)
        );
        assert_eq!(glyph("cursor-agent", AgentIcons::Font), Some("\u{e1ab}"));
        assert_eq!(glyph("open-code", AgentIcons::Font), Some("\u{e1a2}"));
    }

    #[test]
    fn automatic_icons_require_an_available_font() {
        assert_eq!(glyph_for("claude", false), None);
        assert_eq!(glyph_for("claude", true), Some("\u{e1a0}"));
        assert_eq!(canonical_agent("claude-code"), Some("claude"));
        assert_eq!(canonical_agent("fish"), None);
    }

    #[test]
    fn font_detection_finds_installed_files_in_nested_directories() {
        let root = std::env::temp_dir().join(format!("heraldr-fonts-{}", std::process::id()));
        let nested = root.join("vendor");
        std::fs::create_dir_all(&nested).unwrap();
        assert!(!has_font_file(&root, 3));
        std::fs::write(nested.join("Unrelated.ttf"), []).unwrap();
        assert!(!has_font_file(&root, 3));
        std::fs::write(nested.join("HerdrAgentIconsMax-Regular.ttf"), []).unwrap();
        assert!(has_font_file(&root, 3));
        assert!(!has_font_file(&root, 0));
        std::fs::remove_dir_all(root).unwrap();
    }
}

//! Pure naming: program -> icon + display name, and the "[N] " prefix
//! helpers. Nothing here touches herdr or the filesystem beyond loading the
//! icon manifest, so the whole module is unit-testable in isolation.
//!
//! Icons use the compiled `icons.conf` as a baseline and reload the copy in
//! the plugin root when available: one
//! `<glyph> <programs...>` row per icon, full-line `#` comments, and a `* `
//! row naming the fallback glyph for unlisted programs. The manifest is
//! hand-edited only — agent-authored content silently strips U+E000-range
//! Nerd Font glyphs, which is why the glyphs live in data, not Rust source.
//! Test pins below spell glyphs as `\u{...}` escapes for the same reason.

use std::path::Path;

const BUILT_IN_MANIFEST: &str = include_str!("../icons.conf");

/// Label shown at a bare prompt, and for any program that resolves away.
pub const SHELL: &str = "fish";

/// Credential/privilege helpers that never represent what a tab is about.
pub const TRANSIENT: &[&str] = &["op", "ssh-agent", "gpg-agent", "1Password", "sudo"];

/// Instant commands that should not take over the tab name: while one runs,
/// the tab keeps reading as the shell instead of flickering.
pub const IGNORED: &[&str] = &[
    "ls", "lsd", "eza", "exa", "cd", "z", "zoxide", "cat", "bat", "echo", "pwd", "clear", "which",
    "cp", "mv", "rip", "mkdir", "touch", "head", "tail", "wc", "fzf",
];

/// Display renames applied after category resolution.
pub const ALIASES: &[(&str, &str)] = &[("spotify_player", "spotify"), ("notesmd-cli", "notes")];

/// Final name truncation, counted by codepoint.
pub const MAX_NAME_LEN: usize = 20;

#[derive(Debug, Default)]
pub struct Icons {
    rows: Vec<(Vec<String>, String)>,
    fallback: String,
    built_in: bool,
}

impl Icons {
    /// A usable runtime manifest wins; missing or incomplete data falls back
    /// to the copy compiled into the binary.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::runtime_or_built_in(&text),
            Err(_) => Self::built_in(),
        }
    }

    fn runtime_or_built_in(text: &str) -> Self {
        let icons = Self::parse(text);
        if icons.fallback.is_empty() {
            Self::built_in()
        } else {
            icons
        }
    }

    fn built_in() -> Self {
        let mut icons = Self::parse(BUILT_IN_MANIFEST);
        icons.built_in = true;
        icons
    }

    pub fn parse(text: &str) -> Self {
        let mut icons = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split_whitespace();
            let Some(glyph) = fields.next() else { continue };
            let programs: Vec<String> = fields.map(str::to_string).collect();
            if programs.iter().any(|p| p == "*") {
                icons.fallback = glyph.to_string();
            } else if !programs.is_empty() {
                icons.rows.push((programs, glyph.to_string()));
            }
        }
        icons
    }

    pub fn get(&self, program: &str) -> &str {
        self.rows
            .iter()
            .find(|(programs, _)| programs.iter().any(|p| p == program))
            .map(|(_, glyph)| glyph.as_str())
            .unwrap_or(&self.fallback)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_built_in(&self) -> bool {
        self.built_in
    }
}

/// The program a label should be built from. Empty, transient, and
/// instant-command programs all resolve to the shell.
pub fn resolve(program: &str) -> &str {
    if program.is_empty()
        || TRANSIENT.contains(&program)
        || IGNORED.contains(&program)
        || program.starts_with("git-credential-")
    {
        SHELL
    } else {
        program
    }
}

/// The full "<icon> <name>" display label for a program ("" = bare prompt).
pub fn format(program: &str, icons: &Icons) -> String {
    let resolved = resolve(program);
    let name = ALIASES
        .iter()
        .find(|(from, _)| *from == resolved)
        .map(|(_, to)| *to)
        .unwrap_or(resolved);
    let name: String = name.chars().take(MAX_NAME_LEN).collect();
    format!("{} {}", icons.get(resolved), name)
}

/// The leading "[<digits>] " of a label, when present. Only an all-digit
/// bracket counts, so user text like "[wip] foo" is never treated as ours.
pub fn index_prefix(label: &str) -> Option<&str> {
    let rest = label.strip_prefix('[')?;
    let close = rest.find(']')?;
    let digits = &rest[..close];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let prefix_len = 1 + close + 2; // "[", digits, "] "
    label
        .as_bytes()
        .get(prefix_len - 1)
        .filter(|&&b| b == b' ')?;
    Some(&label[..prefix_len])
}

pub fn strip_prefix(label: &str) -> &str {
    index_prefix(label).map_or(label, |p| &label[p.len()..])
}

/// The label an item should have. Positions 1-9 get "[N] base"; anything else
/// (10+, or 0 for a sidebar-hidden row) stays bare since no jump keybind
/// reaches it. `clear` always strips.
pub fn desired(position: usize, base: &str, clear: bool) -> String {
    if !clear && (1..=9).contains(&position) {
        format!("[{position}] {base}")
    } else {
        base.to_string()
    }
}

/// A label is "unnamed" (fair game for first-time auto-naming) when it is
/// empty or a plain integer — herdr's generated tab labels are small integers.
pub fn is_placeholder(label: &str) -> bool {
    label.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Icons {
        Icons::parse(include_str!("../icons.conf"))
    }

    #[test]
    fn missing_runtime_manifest_uses_built_in_icons() {
        let path = std::env::temp_dir().join(format!(
            "heraldr-icons-missing-{}-{}",
            std::process::id(),
            line!()
        ));
        let icons = Icons::load(&path);

        assert!(icons.is_built_in());
        assert_eq!(icons.get("fish"), "\u{f489}");
        assert_eq!(icons.get("yazi"), "\u{f0036}");
        assert_eq!(icons.get("no-such-program"), "\u{f0e7}");
    }

    #[test]
    fn incomplete_runtime_manifest_uses_built_in_icons() {
        let icons = Icons::runtime_or_built_in("\u{f489} fish");

        assert!(icons.is_built_in());
        assert_eq!(icons.get("yazi"), "\u{f0036}");
        assert_eq!(icons.get("no-such-program"), "\u{f0e7}");
    }

    #[test]
    fn resolve_routes_categories_to_the_shell() {
        assert_eq!(resolve(""), "fish");
        assert_eq!(resolve("op"), "fish");
        assert_eq!(resolve("git-credential-osxkeychain"), "fish");
        assert_eq!(resolve("ls"), "fish");
        assert_eq!(resolve("nvim"), "nvim");
    }

    #[test]
    fn manifest_glyphs_pin_exact_codepoints() {
        let icons = manifest();
        assert_eq!(icons.get("nvim"), "\u{e62b}");
        assert_eq!(
            icons.get("wt"),
            "\u{e702}",
            "worktrunk shares the git glyph"
        );
        assert_eq!(icons.get("duckdb"), "\u{f01e5}");
        assert_eq!(icons.get("no-such-program"), "\u{f0e7}", "fallback glyph");
    }

    #[test]
    fn manifest_rows_all_carry_glyphs() {
        let icons = manifest();
        assert!(icons.len() >= 25, "manifest lost rows: {}", icons.len());
        for program in ["fish", "nvim", "claude", "yazi", "cargo"] {
            assert!(!icons.get(program).is_empty(), "empty glyph for {program}");
        }
    }

    #[test]
    fn format_composes_icon_and_simplified_name() {
        let icons = manifest();
        assert_eq!(
            format("nvim", &icons),
            format!("{} nvim", icons.get("nvim"))
        );
        assert_eq!(format("", &icons), format!("{} fish", icons.get("fish")));
        assert_eq!(format("cat", &icons), format!("{} fish", icons.get("fish")));
        assert_eq!(
            format("spotify_player", &icons),
            format!("{} spotify", icons.get("spotify_player"))
        );
        assert_eq!(
            format("somecmd", &icons),
            format!("{} somecmd", icons.get("somecmd"))
        );
    }

    #[test]
    fn format_truncates_names_by_codepoint() {
        let icons = manifest();
        let long = "x".repeat(25);
        let label = format(&long, &icons);
        assert!(label.ends_with(&"x".repeat(20)));
        assert_eq!(label.chars().filter(|&c| c == 'x').count(), 20);
    }

    #[test]
    fn prefix_helpers_round_trip() {
        assert_eq!(strip_prefix("[2] nvim"), "nvim");
        assert_eq!(strip_prefix("[wip] foo"), "[wip] foo");
        assert_eq!(strip_prefix("nvim"), "nvim");
        assert_eq!(strip_prefix("[3] "), "");
        assert_eq!(index_prefix("[7] docs"), Some("[7] "));
        assert_eq!(index_prefix("docs"), None);
        assert_eq!(index_prefix("[12] ten-plus"), Some("[12] "));
    }

    #[test]
    fn desired_prefixes_only_reachable_positions() {
        assert_eq!(desired(2, "x", false), "[2] x");
        assert_eq!(desired(0, "x", false), "x");
        assert_eq!(desired(10, "x", false), "x");
        assert_eq!(desired(2, "x", true), "x");
    }

    #[test]
    fn placeholders_are_empty_or_integer_labels() {
        assert!(is_placeholder(""));
        assert!(is_placeholder("42"));
        assert!(!is_placeholder("nvim"));
        assert!(!is_placeholder("[1] 2"));
    }
}

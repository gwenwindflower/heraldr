use crate::agent_icons;
use crate::config::{Config, TabToken};
use crate::naming::{self, Icons};

pub fn format(
    position: usize,
    program: &str,
    title: Option<&str>,
    icons: &Icons,
    config: &Config,
) -> String {
    let resolved = naming::resolve(program);
    let agent = agent_icons::canonical_agent(resolved);
    let name = naming::display_name(agent.unwrap_or(resolved));
    let logo = if icons.has_override(resolved) {
        None
    } else {
        agent_icons::glyph(resolved, config.agent_icons)
    };
    let icon = logo.unwrap_or_else(|| {
        if agent.is_some() && !icons.has_explicit(resolved) {
            naming::robot_icon()
        } else {
            icons.get(resolved)
        }
    });
    let jump = (1..=9).contains(&position).then(|| format!("[{position}]"));
    let mut label = String::new();
    let mut previous_logo = false;
    for token in &config.tab_label {
        let value = match token {
            TabToken::Jump => jump.as_deref(),
            TabToken::Icon => Some(icon),
            TabToken::Program => Some(name.as_str()),
            TabToken::SessionTitle => title,
        };
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            continue;
        };
        if !label.is_empty() {
            if previous_logo {
                label.push(' ');
            }
            label.push_str(if *token == TabToken::SessionTitle {
                " ⋅ "
            } else {
                " "
            });
        }
        label.push_str(value);
        previous_logo = *token == TabToken::Icon && logo.is_some();
    }
    if label.is_empty() { name } else { label }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentIcons, TabToken};

    fn icons() -> Icons {
        Icons::parse("? *\nS fish\nR claude\nM spotify_player\n")
    }

    #[test]
    fn default_tokens_keep_jump_icon_and_program() {
        let config = Config {
            agent_icons: AgentIcons::None,
            ..Config::default()
        };
        assert_eq!(format(2, "claude", None, &icons(), &config), "[2] R claude");
        assert_eq!(format(10, "fish", None, &icons(), &config), "S fish");
        assert_eq!(
            format(0, "spotify_player", None, &icons(), &config),
            "M spotify"
        );
    }

    #[test]
    fn font_icons_reserve_their_second_terminal_cell() {
        let config = Config {
            agent_icons: AgentIcons::Font,
            ..Config::default()
        };
        assert_eq!(
            format(1, "claude", None, &icons(), &config),
            "[1] \u{e1a0}  claude"
        );
    }

    #[test]
    fn mapped_agents_without_font_icons_use_the_robot_fallback() {
        let config = Config {
            agent_icons: AgentIcons::None,
            ..Config::default()
        };
        assert_eq!(
            format(1, "grok", None, &icons(), &config),
            "[1] \u{f171a} grok"
        );
    }

    #[test]
    fn explicit_user_icons_win_without_hiding_other_agent_logos() {
        let path =
            std::env::temp_dir().join(format!("heraldr-user-icons-{}.conf", std::process::id()));
        std::fs::write(&path, "? *\nC claude\n").unwrap();
        let icons = Icons::load_user(&path);
        let config = Config {
            agent_icons: AgentIcons::Font,
            ..Config::default()
        };
        assert_eq!(format(1, "claude", None, &icons, &config), "[1] C claude");
        assert_eq!(
            format(1, "codex", None, &icons, &config),
            "[1] \u{e1a1}  codex"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tokens_can_be_omitted_and_reordered() {
        let config = Config {
            tab_label: vec![TabToken::Program, TabToken::Jump],
            ..Config::default()
        };
        assert_eq!(format(3, "fish", None, &icons(), &config), "fish [3]");
    }

    #[test]
    fn title_separators_only_exist_between_present_values() {
        let mut config = Config {
            tab_label: vec![TabToken::Program, TabToken::SessionTitle],
            ..Config::default()
        };
        assert_eq!(
            format(1, "claude", Some("Fix login"), &icons(), &config),
            "claude ⋅ Fix login"
        );
        assert_eq!(format(1, "claude", None, &icons(), &config), "claude");
        config.tab_label.reverse();
        assert_eq!(
            format(1, "claude", Some("Fix login"), &icons(), &config),
            "Fix login claude"
        );
    }

    #[test]
    fn absent_tokens_fall_back_to_the_foreground_program() {
        let config = Config {
            tab_label: vec![TabToken::SessionTitle],
            ..Config::default()
        };
        assert_eq!(format(1, "claude", None, &icons(), &config), "claude");
    }
}

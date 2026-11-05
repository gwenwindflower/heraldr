use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn strip_activity(title: &str) -> &str {
    let title = title.trim();
    let Some(first) = title.chars().next() else {
        return title;
    };
    let rest = &title[first.len_utf8()..];
    let activity = matches!(first, '\u{2800}'..='\u{28ff}') || "·✢✳✶✻✽◐◓◑◒".contains(first);
    if activity && (rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        rest.trim()
    } else {
        title
    }
}

pub fn format_suffix(title: Option<&str>, max_width: usize) -> String {
    let Some(title) = title.filter(|_| max_width > 0) else {
        return String::new();
    };
    let cleaned = sanitize(title);
    let title = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return String::new();
    }
    if title.width() <= max_width {
        return format!(" ⋅ {title}");
    }
    let mut end = 0;
    let mut width = 0;
    for (start, grapheme) in title.grapheme_indices(true) {
        width += grapheme.width();
        if width > max_width - 1 {
            break;
        }
        end = start + grapheme.len();
    }
    format!(" ⋅ {}…", &title[..end])
}

#[derive(Clone, Copy)]
enum Escape {
    Text,
    Start,
    Intermediate,
    Csi,
    String,
    StringEnd,
}

fn sanitize(title: &str) -> String {
    let mut output = String::with_capacity(title.len().min(4096));
    let mut escape = Escape::Text;
    for c in title.chars().take(4096) {
        escape = match (escape, c) {
            (_, '\u{1b}') if !matches!(escape, Escape::String | Escape::StringEnd) => Escape::Start,
            (Escape::Text, '\u{9b}') | (Escape::Start, '[') => Escape::Csi,
            (Escape::Text, '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}')
            | (Escape::Start, ']' | 'P' | 'X' | '^' | '_') => Escape::String,
            (Escape::Text, c) => {
                if c.is_whitespace() {
                    output.push(' ');
                } else if !c.is_control() {
                    output.push(c);
                }
                Escape::Text
            }
            (Escape::Start | Escape::Intermediate, '\u{20}'..='\u{2f}') => Escape::Intermediate,
            (Escape::Start | Escape::Intermediate, _)
            | (Escape::Csi, '\u{40}'..='\u{7e}')
            | (Escape::String | Escape::StringEnd, '\u{7}' | '\u{9c}')
            | (Escape::StringEnd, '\\') => Escape::Text,
            (Escape::Csi, _) => Escape::Csi,
            (Escape::String | Escape::StringEnd, '\u{1b}') => Escape::StringEnd,
            (Escape::String | Escape::StringEnd, _) => Escape::String,
        };
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_blank_titles_have_no_separator() {
        for title in [None, Some(""), Some(" \n\t ")] {
            assert_eq!(format_suffix(title, 28), "");
        }
    }

    #[test]
    fn title_whitespace_collapses_to_one_line() {
        assert_eq!(
            format_suffix(Some("  Repair\n login\t flow  "), 28),
            " ⋅ Repair login flow"
        );
    }

    #[test]
    fn escape_sequences_and_control_bytes_do_not_enter_the_label() {
        let title = "\u{1b}[31mRepair\u{1b}[0m \u{1b}]8;;https://example.com\u{1b}\\login\u{1b}]8;;\u{7} flow\u{0}\u{8}";
        assert_eq!(format_suffix(Some(title), 28), " ⋅ Repair login flow");
    }

    #[test]
    fn unterminated_terminal_commands_drop_their_payload() {
        for title in [
            "Repair\u{1b}]0;injected",
            "Repair\u{1b}[31",
            "Repair\u{1b}Ppayload",
        ] {
            assert_eq!(format_suffix(Some(title), 28), " ⋅ Repair");
        }
    }

    #[test]
    fn truncation_budget_includes_the_ellipsis() {
        assert_eq!(format_suffix(Some("Fix login"), 9), " ⋅ Fix login");
        assert_eq!(format_suffix(Some("Fix login"), 8), " ⋅ Fix log…");
        assert_eq!(format_suffix(Some("Fix login"), 1), " ⋅ …");
        assert_eq!(format_suffix(Some("Fix login"), 0), "");
    }

    #[test]
    fn truncation_counts_terminal_columns_and_keeps_graphemes() {
        assert_eq!(format_suffix(Some("修復登入流程"), 7), " ⋅ 修復登…");
        assert_eq!(format_suffix(Some("e\u{301}clair"), 4), " ⋅ e\u{301}cl…");
        assert_eq!(format_suffix(Some("👩‍💻 login"), 4), " ⋅ 👩‍💻 …");
    }

    #[test]
    fn raw_titles_strip_one_recognized_activity_glyph() {
        assert_eq!(strip_activity(" ⠋ Repair login "), "Repair login");
        assert_eq!(strip_activity("✳ Repair login"), "Repair login");
        assert_eq!(strip_activity("⠋ ⠙ Repair login"), "⠙ Repair login");
        assert_eq!(strip_activity("✳   "), "");
        assert_eq!(strip_activity("★ Repair login"), "★ Repair login");
        assert_eq!(strip_activity("✳Repair login"), "✳Repair login");
    }
}

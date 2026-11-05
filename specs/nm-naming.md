# Tab naming

## Goals

Owned tabs follow their foreground program across process transitions. Temporary failures during credential or privilege prompts leave automatic naming recoverable. Manual tab names remain under the user's control.

## Requirements

- **nm-R001** — If foreground process inspection fails or finds no process-group leader, the watcher retains the tab's label and resumes automatic naming when inspection succeeds.
- **nm-R002** — If a tab rename fails, Heraldr retains the last confirmed ownership label and retries automatic naming on a subsequent pass.
- **nm-R003** — If a focused tab rename fails, the watcher continues polling without requiring a restart or another event.
- **nm-R004** — When the user manually renames an owned tab, automatic naming remains disabled across subsequent process transitions until the user resets or clears the label.
- **nm-R005** — Naming ownership belongs to one Herdr session; another session cannot change or prune its tab records, including when tab IDs match.
- **nm-R006** — When a session first uses a dedicated ownership file, it preserves the records from an existing shared ownership file without modifying that shared file.
- **nm-R007** — Plugin invocations store ownership and watcher locks in `HERDR_PLUGIN_STATE_DIR`, inheriting existing ownership from the standalone state directory when needed.
- **nm-R008** — If the event subscription disconnects while the session remains reachable, the watcher reconnects and continues automatic naming without another plugin event.
- **nm-R009** — User icons in `HERDR_PLUGIN_CONFIG_DIR/icons.conf` take precedence over shipped icons and reload during reconciliation.
- **nm-R010** — Supported foreground agents use their Herdr Agent Icons Max logo when the font is detected, and otherwise retain their configured Nerd Font icon.
- **nm-R011** — An explicit user icon row takes precedence over an automatically selected agent logo.
- **nm-R012** — The `agent_icons` preference accepts `auto`, `font`, or `none`; agent logos inherit Herdr's tab text color.
- **nm-R013** — The `tab_label` preference composes `$jump`, `$icon`, `$program`, and `$session_title` in the configured order; its default contains `$jump`, `$icon`, and `$program`.
- **nm-R014** — Session titles contain no terminal controls and fit `session_title_max_width` terminal columns, defaulting to 28, including an ellipsis when truncated without splitting a grapheme.
- **nm-R015** — Session-title changes update owned tabs, and leaving an agent removes its title while preserving existing process tracking and jump-number rules.
- **nm-R016** — Changes to plugin preferences reload without restarting the watcher; invalid preferences report the config path and retain the running watcher's last usable settings.
- **nm-R017** — Manual tab names remain unchanged by agent icons and session titles until the user resets or clears the name.
- **nm-R018** — Missing token values disappear with their separators; an otherwise empty automatic label falls back to the foreground program.
- **nm-R019** — `$session_title` appears only for the pane's current foreground agent, separated from preceding content by ` ⋅ `, and inherits Herdr's tab text style.

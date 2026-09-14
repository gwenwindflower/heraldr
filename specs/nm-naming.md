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

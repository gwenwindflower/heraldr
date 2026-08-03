# Livery

[![CI](https://github.com/gwenwindflower/livery/actions/workflows/ci.yml/badge.svg)](https://github.com/gwenwindflower/livery/actions/workflows/ci.yml)

Livery gives [Herdr](https://herdr.dev) tabs and sidebar rows compact, live context:

- Tabs follow the foreground program and read as `[N] <icon> <program>`.
- Workspace rows expose their jump number through Herdr's `$jump` display token.
- Agent rows expose the repository and linked worktree through `$space` and `$worktree`.
- Manual tab names always win.

Livery supports Linux and macOS. Its icons require a terminal font with Nerd Font glyphs.

## Install

You need [Herdr](https://herdr.dev/docs/install/) 0.7.5 or later, Git, and a compatible [Rust toolchain with Cargo](https://www.rust-lang.org/tools/install). [Cargo Binstall](https://github.com/cargo-bins/cargo-binstall#installation) is optional, but makes installation much faster when Livery has a release artifact for your platform.

```bash
herdr plugin install gwenwindflower/livery
```

Herdr previews the plugin commands and enables Livery for every session owned by the current user. The installer reuses a matching `livery` binary from the checkout or your `PATH`, then tries Cargo Binstall, then builds from source. A workspace, tab, or pane event starts the per-session watcher.

Verify the installation:

```bash
herdr plugin list --plugin livery
```

### Update

Stop the running watcher, then reinstall the GitHub plugin to refresh both its managed checkout and its binary:

```bash
herdr plugin action invoke livery.clear
herdr plugin install gwenwindflower/livery
```

The next workspace, tab, or pane event starts the replacement watcher. The installer compares binaries against the version in Livery's Cargo manifest before reusing them. Herdr refuses to replace a local link with a GitHub install; [switch back to the released plugin](#switch-between-local-and-released-livery) first.

### Install the binary yourself

Install the latest release artifact with Cargo Binstall:

```bash
cargo binstall livery --git https://github.com/gwenwindflower/livery
```

Or compile and install it from source:

```bash
cargo install --git https://github.com/gwenwindflower/livery --locked
```

These commands put `livery` on your `PATH`; `herdr plugin install gwenwindflower/livery` still registers the manifest and will reuse the binary when its version matches.

## Actions

A manual tab rename opts that tab out of automatic naming. Adopt the current tab again with:

```bash
herdr plugin action invoke livery.reset
```

Before uninstalling, stop the watcher and remove Livery's display metadata:

```bash
herdr plugin action invoke livery.clear
herdr plugin uninstall livery
```

## Local development

Build the checkout before linking it because `plugin link` does not run manifest build commands:

```bash
cargo build --locked --release
herdr plugin link .
```

The link points directly at the checkout, so code and `icons.conf` stay local. Rebuild after Rust changes; relink only when `herdr-plugin.toml` changes. Stop the running watcher before replacing its binary:

```bash
herdr plugin action invoke livery.clear
cargo build --locked --release
```

### Switch between local and released Livery

Replace a GitHub install with the local checkout:

```bash
herdr plugin action invoke livery.clear
herdr plugin uninstall livery
cargo build --locked --release
herdr plugin link .
```

Test a release after local development:

```bash
herdr plugin action invoke livery.clear
herdr plugin unlink livery
herdr plugin install gwenwindflower/livery
```

`unlink` leaves the checkout alone. `uninstall`, `unlink`, and reinstall also leave Livery's user config in place. Print its stable location with:

```bash
herdr plugin config-dir livery
```

With Herdr's default paths, future Livery config lives under `~/.config/herdr/plugins/config/livery`, separate from either source checkout.

### Checks

Run the local checks with:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
./tests/install-binary.sh
```

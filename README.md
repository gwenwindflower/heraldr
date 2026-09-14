# Heraldr

[![CI](https://github.com/gwenwindflower/heraldr/actions/workflows/ci.yml/badge.svg)](https://github.com/gwenwindflower/heraldr/actions/workflows/ci.yml)

Heraldr gives [Herdr](https://herdr.dev) tabs and sidebar rows compact, live context:

- Tabs follow the foreground program and read as `[N] <icon> <program>`.
- Workspace rows expose their jump number through Herdr's `$jump` display token.
- Agent rows expose the repository and linked worktree through `$space` and `$worktree`.
- Manual tab names always win.

Heraldr supports Linux and macOS. Its icons require a terminal font with Nerd Font glyphs.

## Install

You need [Herdr](https://herdr.dev/docs/install/) 0.7.5 or later, Git, and a compatible [Rust toolchain with Cargo](https://www.rust-lang.org/tools/install). [Cargo Binstall](https://github.com/cargo-bins/cargo-binstall#installation) is optional, but installs a release artifact much faster than compiling from source. Install the plugin:

```bash
herdr plugin install gwenwindflower/heraldr
```

Herdr previews the plugin commands, then the build hook installs the matching Heraldr release on your `PATH`. It tries Cargo Binstall first when available, then compiles from source with Cargo. Plugin events and actions invoke `heraldr` directly; a workspace, tab, or pane event starts the per-session watcher.

Verify the installation:

```bash
herdr plugin list --plugin heraldr
```

> [!TIP]
> If you have mise installed, you can also use [mise to manage rust tools](https://mise.jdx.dev/lang/rust.html), it will use `cargo-binstall` to install tools by default if added as a tool in your `mise.toml` file. If you'd prefer to have mise manage your cargo bin tools, run `mise use -g cargo:heraldr`. The plugin install will not install the binary if it's already available on `PATH`. We use mise for development and CI, so this is convenient if you also want to develop Heraldr locally.

### Update

Stop the running watcher, then reinstall the plugin. Its build hook updates the binary on your `PATH`:

```bash
herdr plugin action invoke heraldr.clear
herdr plugin install gwenwindflower/heraldr
```

The next workspace, tab, or pane event starts the replacement watcher. Herdr refuses to replace a local link with a GitHub install; [switch back to the released plugin](#switch-between-local-and-released-heraldr) first.

## Actions

A manual tab rename opts that tab out of automatic naming. Adopt the current tab again with:

```bash
herdr plugin action invoke heraldr.reset
```

Before uninstalling, stop the watcher and remove Heraldr's display metadata:

```bash
herdr plugin action invoke heraldr.clear
herdr plugin uninstall heraldr
```

## Local development

Development runs on [mise](https://mise.jdx.dev), which owns both the toolchain and the task list. Clone the repository, then:

```bash
mise trust
mise install
```

That installs the linters and release tooling `mise.toml` declares, so your machine and CI run the same tools. Rust comes from rustup on your `PATH`; `rust-toolchain.toml` selects stable with clippy and rustfmt. Herdr itself is the other prerequisite mise does not manage.

Already have some of those on your `PATH`? Copy `mise.local.toml.example` to `mise.local.toml` (gitignored) and list them under `disable_tools`. mise then skips installing them here and tasks use whatever `command -v` finds; CI installs whatever `mise.toml` resolves, so keep yours current.

`mise tasks` lists every task with its description; `mise tasks info <task>` prints one task's full definition, arguments, and source. Tasks are grouped by prefix — `dev:`, `herdr:`, `lint:`, `test:`, `ci-audit:`, `version:`, `release:`, `repo:` — and the common ones carry single-letter aliases.

Put the checkout in front of Herdr:

```bash
mise run dev:reload
```

That stops the running watcher, installs the checkout's binary on your `PATH`, and links its manifest. The link points directly at the checkout, so `herdr-plugin.toml` and `icons.conf` stay local. `icons.conf` edits apply live; the binary carries the same map as a fallback when the checkout is unavailable. Rerun `dev:reload` after Rust changes.

`mise run dev` runs local checks and tests, then installs a fresh build from this checkout with `cargo install --path .` and links it into Herdr. Local linking does not run the manifest's release installer or Cargo Binstall.

### Switch between local and released Heraldr

`dev:local` replaces a GitHub install with the checkout. `dev:released` swaps back to the published plugin, for testing a release the way a user receives it.

`herdr:unlink` leaves the checkout alone, and unlinking, uninstalling, and reinstalling all leave Heraldr's user config in place. Print its stable location with `herdr plugin config-dir heraldr` — with Herdr's default paths it lives under `~/.config/herdr/plugins/config/heraldr`, separate from either source checkout.

### Checks

`mise run check` is the full local gate. CI installs mise and runs these same tasks, so a task definition is the only place a check lives.

`wt merge` runs one gate after rebasing onto the target: `check` for a feature branch, or `release:check` (local checks plus workflow audits) for the default branch. Commit hooks still check staged files. These gates use plain Cargo output and never install Heraldr on your `PATH`; installer tests simulate Cargo and Binstall inside temporary directories.

### Pretty tasks

There are local interactive development versions of the build and test tasks that use [cargo-pretty](https://github.com/romancitodev/cargo-pretty) for rich output. You'll need to install cargo pretty for them to work (`cargo binstall cargo-pretty-build`).

```bash
mise run dev:build
mise run b
mise run dev:test
mise run t
```

## Releases

`Cargo.toml` is the source of truth for Heraldr's version. `version:write` synchronizes `Cargo.lock` and `herdr-plugin.toml` to it, `version:check` reports drift, and release notes live in the GitHub Release rather than a separate changelog file.

Releases run from a clean local `main` with `gh` already authenticated. All tags are `vMAJOR.MINOR.PATCH`. The policy in `cliff.toml` begins at `v0.0.1`, keeps `0.0.x` releases on patch bumps, and does not promote the project to `0.1.0` or `1.0.0` automatically.

`mise run release:rehearse` is the dry run: it exercises every read-only step against the current checkout and prints the notes that would ship, without touching a file, a branch, or GitHub. `mise run release` then runs the pipeline end to end — preflight, version bump, the full gate, the release commit, push, and publish. The two steps that leave your machine, `release:push` and `release:create`, each require a confirmation that defaults to no. Every step is also runnable on its own, so a pipeline that stops partway can be resumed from where it stopped.

GitHub creates the tag at the default-branch head when it publishes the release. Do not create or push a separate tag. The `Release build` workflow then verifies the tag against Cargo, builds each supported target, and attaches archives and checksums. `mise run release:verify` inspects the result.

Rerun failed release jobs from their original event so they retain permission to upload assets: `gh run rerun <run-id> --failed`. To rebuild an existing tag for diagnosis without changing the release, run `gh workflow run release-build.yml -f tag=vX.Y.Z` — it keeps outputs as workflow artifacts.

## Issues vs. Discussions

Discussions are for ideas, questions, and "what if Heraldr did X?". Issues are for concrete, reproducible change requests: a bug with a repro, a missing option with a clear shape, a doc inaccuracy. Start in Discussions if you're not sure.

## License

MIT. See [LICENSE](./LICENSE).

# Heraldr

## Goals

Heraldr gives Herdr tabs and sidebar rows compact, live context without the user naming anything. Tabs follow the foreground program and read as `[N] <icon> <program>`; workspace rows expose their jump number through Herdr's `$jump` token; agent rows expose the repository and linked worktree through `$space` and `$worktree`. A manual rename always wins and opts that tab out until the user adopts it back. Heraldr runs on Linux and macOS with a Nerd Font terminal. Non-goals: theming beyond labels and icons, persisting layout, and any behavior that requires a Herdr version older than the manifest's minimum.

## Vocabulary

- **Watcher**: the per-session process that subscribes to Herdr events and reconciles labels.
- **Kick**: the idempotent event entrypoint that starts a watcher when none holds the session.
- **Owned tab**: a tab whose label Heraldr last set; tracked per session so manual renames are respected.
- **Relabel**: adopting tabs into automatic naming, manual names included, for one tab, one workspace, or the whole session.
- **Restart**: replacing the session's watcher with a fresh one from the `heraldr` on `PATH`, keeping labels and ownership.
- **Clear**: stopping the watcher and removing every label and token Heraldr set.

## Domain specs

- @specs/nm-naming.md
- @specs/dev-release.md

## Requirements

- **R001** — Always: `heraldr --version` reports the version `Cargo.toml` declares, and the manifest declares the same version.

## Backlog

- Extend the naming spec with label format, token semantics, and icon fallback.
- Document the watcher, reconcile, snapshot, and rpc boundaries in `docs/architecture.md`.
- Unit coverage for `rpc.rs`, `snapshot.rs`, and `watch.rs`.
- Linux `aarch64` and musl targets once a user asks for them.

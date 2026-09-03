# Heraldr

A Herdr plugin that keeps tab and sidebar-row names in sync with live session context. Rust, distributed as a single binary through Herdr's plugin installer.

## Run work through tasks

[mise](https://mise.jdx.dev) owns both the toolchain and the task list. `mise tasks` lists every task with its description; `mise tasks info <task>` prints one task's definition, arguments, and source.

Prefer a task over the command it wraps. If you need something the tasks do not cover, add a task rather than running a one-off — a command worth running twice belongs in the task list. CI installs mise and runs these same tasks, so a task definition is the only place a check lives. `mise run check` is the full local gate.

Task scripts live in `mise-tasks/`, grouped into directories that become the `version:` and `release:` prefixes. Each is ordinary bash that runs standalone — `./mise-tasks/version/check` works without mise — and they compose by calling each other through those paths. `mise.toml` holds the one-line wrappers, the pipelines, and the tool versions.

Two mise details worth knowing when editing tasks. `depends` runs in parallel, so anything order-sensitive belongs in a sequential `run` array instead. And `deny_net`/`deny_write` are only honored on TOML tasks; mise warns and ignores them in `#MISE` file-task headers.

## Releases are human-gated

Do not run `release`, `release:push`, or `release:create`. The last two carry mise `confirm` gates that default to no, and they push commits and create GitHub releases.

Everything else in the pipeline is safe to run. `release:rehearse` is the dry run: it exercises every read-only step and prints the notes that would ship, without touching a file, a branch, or GitHub. Run it when the project looks ready, report what it says, and stop.

`Cargo.toml` is the sole source of truth for the version. Never hand-edit the version in `Cargo.lock` or `herdr-plugin.toml`; report drift with `version:check` and repair it with `version:sync`, which only ever writes those two files. `version:bump` moves Cargo's own version and belongs to a release, not to ordinary work.

## Workflow changes

Any edit under `.github/workflows/` must keep every action pinned to a commit SHA with a trailing version comment, and must pass `mise run ci-audit` (zizmor and pinact).

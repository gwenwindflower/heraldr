# Heraldr

A Herdr plugin that keeps tab and sidebar-row names in sync with live session context. Rust, distributed as a single binary through Herdr's plugin installer from `gwenwindflower/heraldr`.

## Run work through tasks

[mise](https://mise.jdx.dev) owns the toolchain and the task list. `mise tasks` lists every task with its description; `mise tasks info <task>` prints one task's definition, arguments, and source. Prefer a task over the command it wraps, and add a task rather than running a one-off. CI installs mise and runs these same tasks, so a task definition is the only place a check lives. `mise run check` is the full local gate.

Task scripts live in `mise-tasks/`, grouped into directories that become the `version:`, `release:`, and `repo:` prefixes. Each is plain bash that runs standalone, and they compose by calling each other through those paths. `mise.toml` holds one-line wrappers, pipelines, and tool versions.

`depends` runs in parallel, so order-sensitive steps belong in a sequential `run` array. `deny_net` and `deny_write` are honored only on TOML tasks.

Herdr commands (`herdr:*`, `dev:*`) need the Herdr socket, which sandboxed agents cannot reach; ask the user to run them.

## Releases are human-gated

Never run `release`, `release:push`, or `release:create`. They push commits and create public GitHub releases behind mise `confirm` gates that default to no. `release:rehearse` is the dry run: it exercises every read-only step and prints the notes that would ship. Run it when the project looks ready, report what it says, and stop.

`release:bootstrap-crate` is the human-confirmed first crates.io publication. `release:publish-crate` runs only in the release workflow with OIDC credentials, after binary uploads. Never invoke either publication task as a check; `test:crate` runs `cargo package` to verify the packaged source in `check` and CI. `CRATES_IO_PUBLISHING=true` enables the OIDC job after the first crate exists and its trusted publisher is configured.

`Cargo.toml` is the version's source of truth, read through `version:read`. `version:write` derives `Cargo.lock` and `herdr-plugin.toml` from it; never hand-edit those versions. Report drift with `version:check` and repair it with `version:sync`. `version:bump` belongs to a release, not to ordinary work.

## Hooks guard commits, tasks guard merges

Use a Worktrunk worktree for development and integrate locally with `wt merge`. Mise checks, prek commit hooks, and Worktrunk merge hooks provide local CI. Open a pull request only when the user explicitly requests one; hosted-runner verification alone is not permission to open a PR.

prek runs file hygiene on every commit (staged files only) and rejects commit subjects git-cliff cannot parse; `mise run hooks:install` wires it into a fresh clone. `wt merge` runs one gate after the rebase: `release:check` when its target is Worktrunk's default branch, otherwise `check`. Never commit with `--no-verify`; fix what the hook reports.

Interactive Cargo tasks live under `dev:`; `test:*` is safe for unattended gates and CI. Installer tests use fake Cargo/Binstall in temporary directories. Local reloads use `cargo install --path .` and `herdr plugin link`, which does not execute manifest build hooks.

Shell tasks and tests use Bash and standard Unix utilities; use `grep -F` for literals and `grep -E` for regular expressions. Developer search preferences do not make tools such as ripgrep runtime prerequisites. Declare nonstandard task dependencies in mise.

## Workflow changes

Every `uses:` under `.github/workflows/` stays pinned to a commit SHA with a trailing version comment, and every change passes `mise run ci-audit`.

## Plugin contract

Manifest entrypoints call `heraldr` from `PATH`; the build hook (`scripts/install-binary.sh`) owns putting the exact version there. `tests/plugin-manifest.sh` and `tests/install-binary.sh` enforce both. Icons live in `icons.conf`; the binary carries a compiled copy as a fallback.

## Planning

This project uses SPOT: `SPEC.md` and `specs/` hold requirements with stable IDs, `TODO.md` holds active Phases, `DONE.md` is the ledger of shipped work. The `projects` rule and the `spot-project-management` skill define the system. Commit bodies carry `Completes <Objective> in Phase N` and `Closes Phase N` lines after any body bullets and before trailers.

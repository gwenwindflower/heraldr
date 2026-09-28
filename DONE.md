# Heraldr DONE

Shipped Phases, newest first. Each entry keeps the Phase header, Objectives, and Tasks verbatim with the boxes checked, followed by a short narrative of decisions and surprises.

## Phase 6: Portable CI and repository provisioning ✅

**Requirements**: dev-R011, dev-R023, dev-R024

### Portable workflow plumbing

- [x] Replace ripgrep in shell tests and verify without it
- [x] Remove unused Homebrew provisioning and formula generation
- [x] Restrict required-check discovery to CI and test the generated ruleset
- [x] Run gates and record the toolchain simplification audit

Hosted test runners exposed an undeclared ripgrep dependency. Shell suites use standard grep and Bash matching, and all suites passed with ripgrep disabled. Ruleset discovery reads only the latest completed CI push run on main, excluding release jobs attached to the same commit; a regression test checks the generated policy without touching GitHub. The existing jq dependency is declared in mise.

Removed unused Homebrew scaffolding and replaced dry-run publication with Cargo's package verification. `release:check` passed, including 35 Rust tests, shell suites, optimized and packaged builds, and workflow audits. The tooling audit retained the shared mise tasks; future distribution-tool research lives in the tool system's vault notes.

## Phase 5: Crates.io release publishing ✅

**Requirements**: dev-R019, dev-R020, dev-R021, dev-R022

### Verified crate publication

- [x] Add optimized build and crate package verification to local and CI gates
- [x] Test release source and artifact checks before crate publication
- [x] Add confirmed first publication and OIDC release tasks
- [x] Verify gates and document the first-release setup

The existing `test:*` task group carries optimized builds and packaged-crate compilation into CI and both merge gates. The package include list ships only Rust source, Rust tests, icons, metadata, and user documentation. Dry runs accept uncommitted changes so they can verify the release pipeline's version bump; real publication requires a clean checkout at the release tag and all eight binary assets on GitHub.

The confirmed bootstrap task uses local Cargo credentials because crates.io requires an initial token-based publication. `CRATES_IO_PUBLISHING` gates the OIDC job until the crate and trusted publisher exist. The job runs after asset upload, only for release events, with the `release` environment and a SHA-pinned official authentication action. `release:check` passed, including the publication failure cases, 35 Rust tests, source packaging, and workflow audits. Initial publication and external OIDC configuration remain human steps in Phase 1.

## Phase 3: Merge and development task isolation ✅

**Requirements**: dev-R014, dev-R015, dev-R016, dev-R017, dev-R018

### Noninteractive merge gates and local development

- [x] Verify automated task selection excludes interactive commands and duplicate suites
- [x] Select the merge gate using Worktrunk's target and default branch variables
- [x] Clarify simulated installer output and verify local development installs the checkout
- [x] Run the gates and preview both merge targets without merging

The merge ran lint twice because its pre-commit hook and release gate both included it. The test wildcard also selected `test:pretty`, running Rust tests twice and putting an interactive terminal task inside the blocking gate. That task was the likely cause of the reported stall; the original terminal stall was not reproduced. Pretty tasks live under `dev:` with unique `b`/`t` aliases. Prek defaults file checks to `pre-commit`, preventing another sweep during `commit-msg`.

[Worktrunk's hook context](https://worktrunk.dev/hook/#template-variables) supplies both `target` and `default_branch`, so a single pre-merge hook selects `check` or `release:check` without assuming `main`. Both gates passed in about four seconds, including 31 Rust tests and the shell suites. Dry-run previews covered feature targets and default branches named `main` and `trunk`; no merge ran.

Installer tests replace Cargo and Binstall in temporary directories. Their output labels the simulation and prints captured installer logs on failure. Local development uses `cargo install --path .` before linking; [Herdr does not run manifest build commands for local links](https://github.com/herdrdev/herdr/blob/master/docs/next/website/src/content/docs/plugins.mdx#build-commands). The dev command sequence was inspected without reloading the live plugin. Workflow audits accept an explicit `GITHUB_TOKEN` or anonymous API access rather than extracting stored credentials through `gh`.
